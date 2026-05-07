//! 游戏进程启动和监控逻辑

use crate::errors::LauncherError;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Emitter;

/// 游戏进程最大运行时间（24 小时）
const MAX_GAME_RUNTIME: Duration = Duration::from_secs(24 * 60 * 60);

/// 收集的 stderr 最大行数
const MAX_STDERR_LINES: usize = 50;

/// 启动并监控游戏进程
pub fn spawn_and_monitor_process(
    java_path: &str,
    final_args: Vec<String>,
    working_dir: &Path,
    window: tauri::Window,
) -> Result<(), LauncherError> {
    let mut command = Command::new(java_path);
    command.args(&final_args);
    command.current_dir(working_dir);

    // 在 Windows 上隐藏命令行窗口
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW = 0x08000000
        command.creation_flags(0x08000000);
    }

    let _ = window.emit("log-debug", format!("最终启动命令: {:?}", command));
    window.emit("launch-command", format!("{:?}", command))?;

    // 启动游戏进程
    let child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let pid = child.id();
    let _ = window.emit("log-debug", format!("游戏已启动，PID: {}", pid));

    // 发送游戏启动成功的事件到前端
    window.emit("minecraft-launched", format!("游戏已启动，PID: {}", pid))?;

    // 在后台线程中监控游戏进程（带超时）
    spawn_monitor_thread(child, window, pid, working_dir.to_path_buf());

    Ok(())
}

/// 启动监控线程（带超时机制）
fn spawn_monitor_thread(mut child: Child, window: tauri::Window, pid: u32, working_dir: std::path::PathBuf) {
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let stderr = child.stderr.take().expect("Failed to open stderr");

    let is_running = Arc::new(AtomicBool::new(true));
    let stderr_lines: Arc<std::sync::Mutex<Vec<String>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let window_clone = window.clone();
    let is_running_stdout = is_running.clone();

    // 启动 stdout 读取线程
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(l) = line {
                let _ = window_clone.emit("log-debug", l);
            }
            if !is_running_stdout.load(Ordering::SeqCst) {
                break;
            }
        }
    });

    let window_clone_err = window.clone();
    let is_running_stderr = is_running.clone();
    let stderr_lines_clone = stderr_lines.clone();
    // 启动 stderr 读取线程
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(l) = line {
                let _ = window_clone_err.emit("log-error", l.clone());
                let mut buf = stderr_lines_clone.lock().unwrap();
                if buf.len() >= MAX_STDERR_LINES {
                    buf.remove(0);
                }
                buf.push(l);
            }
            if !is_running_stderr.load(Ordering::SeqCst) {
                break;
            }
        }
    });

    std::thread::spawn(move || {
        let start_time = Instant::now();
        let is_running_main = is_running.clone();

        // 启动超时检查线程
        let is_running_timeout = is_running.clone();
        let window_timeout = window.clone();
        let timeout_thread = std::thread::spawn(move || {
            while is_running_timeout.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_secs(60));
                
                if !is_running_timeout.load(Ordering::SeqCst) {
                    break;
                }

                let elapsed = start_time.elapsed();
                if elapsed > MAX_GAME_RUNTIME {
                    let _ = window_timeout.emit(
                        "log-warning",
                        format!(
                            "游戏运行时间超过 {} 小时，监控线程将停止",
                            MAX_GAME_RUNTIME.as_secs() / 3600
                        ),
                    );
                    break;
                }
            }
        });

        // 等待进程结束
        match wait_for_process_with_timeout(&mut child, MAX_GAME_RUNTIME) {
            Ok(Some(status)) => {
                is_running_main.store(false, Ordering::SeqCst);
                // 短暂等待 stderr 线程读取完剩余输出
                std::thread::sleep(Duration::from_millis(500));
                let stderr_buf = stderr_lines.lock().unwrap().clone();
                handle_process_exit(status, &window, &stderr_buf, &working_dir);
            }
            Ok(None) => {
                is_running_main.store(false, Ordering::SeqCst);
                let _ = window.emit(
                    "log-warning",
                    format!("游戏进程 (PID: {}) 运行超时，停止监控", pid),
                );
                let _ = window.emit(
                    "minecraft-timeout",
                    format!("游戏运行超过 {} 小时，监控已停止", MAX_GAME_RUNTIME.as_secs() / 3600),
                );
            }
            Err(e) => {
                is_running_main.store(false, Ordering::SeqCst);
                let _ = window.emit("log-error", format!("监控游戏进程时出错: {}", e));
                let _ = window.emit("minecraft-error", format!("监控游戏进程时出错: {}", e));
            }
        }

        let _ = timeout_thread.join();
    });
}

/// 等待进程结束（带超时）
fn wait_for_process_with_timeout(
    child: &mut Child,
    timeout: Duration,
) -> Result<Option<ExitStatus>, std::io::Error> {
    let start = Instant::now();

    loop {
        match child.try_wait()? {
            Some(status) => return Ok(Some(status)),
            None => {
                if start.elapsed() > timeout {
                    return Ok(None);
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    }
}

/// 查找 JVM 崩溃日志文件 (hs_err_pid*.log)
fn find_hs_err_log(working_dir: &Path) -> Option<String> {
    if let Ok(entries) = fs::read_dir(working_dir) {
        let mut latest: Option<(std::time::SystemTime, String)> = None;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("hs_err_pid") && name_str.ends_with(".log") {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if latest.as_ref().map_or(true, |(t, _)| modified > *t) {
                            if let Ok(content) = fs::read_to_string(entry.path()) {
                                latest = Some((modified, content));
                            }
                        }
                    }
                }
            }
        }
        return latest.map(|(_, content)| content);
    }
    None
}

/// 处理进程退出
fn handle_process_exit(
    status: ExitStatus,
    window: &tauri::Window,
    stderr_lines: &[String],
    working_dir: &Path,
) {
    let exit_code = status.code();
    let _ = window.emit(
        "log-debug",
        format!("游戏进程退出，状态码: {:?}", exit_code),
    );

    // 如果游戏以非零退出码退出，发送错误事件
    if exit_code.unwrap_or(-1) != 0 {
        let mut error_msg = format!("游戏以非零状态码退出: {:?}", exit_code);

        if !stderr_lines.is_empty() {
            error_msg.push_str("\n\n--- 游戏错误输出 ---\n");
            for line in stderr_lines {
                error_msg.push_str(line);
                error_msg.push('\n');
            }
        }

        // 尝试读取 JVM 崩溃日志
        if let Some(hs_err) = find_hs_err_log(working_dir) {
            error_msg.push_str("\n--- JVM 崩溃日志 (hs_err_pid) ---\n");
            // 只取前 30 行，避免消息过长
            let lines: Vec<&str> = hs_err.lines().take(30).collect();
            error_msg.push_str(&lines.join("\n"));
            if hs_err.lines().count() > 30 {
                error_msg.push_str("\n... (日志已截断)");
            }
        }

        if exit_code == Some(-1) || exit_code == Some(1) {
            error_msg.push_str("\n\n--- 常见原因 ---\n");
            error_msg.push_str("• Java 版本不兼容（如用 Java 8 启动 1.17+ 版本）\n");
            error_msg.push_str("• 缺少 Natives 库（LWJGL）或 Natives 与 Java 版本不兼容\n");
            error_msg.push_str("• 缺少关键库文件或主游戏 JAR\n");
            error_msg.push_str("• 内存分配超出系统可用内存\n");
            error_msg.push_str("• 显卡驱动问题（游戏窗口创建失败）\n");
            error_msg.push_str("建议：检查 Java 版本是否匹配，尝试删除 versions/<版本>/natives 目录后重新启动，或尝试修复游戏文件。");
        }

        let _ = window.emit("minecraft-error", error_msg);
    }

    // 发送游戏退出事件
    let _ = window.emit(
        "minecraft-exited",
        format!("游戏已退出，状态码: {:?}", exit_code),
    );
}

