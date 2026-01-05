//! 游戏进程启动和监控逻辑

use crate::errors::LauncherError;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Emitter;

/// 游戏进程最大运行时间（24 小时）
const MAX_GAME_RUNTIME: Duration = Duration::from_secs(24 * 60 * 60);

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

    // 启动游戏进程但不等待它结束
    let child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let pid = child.id();
    let _ = window.emit("log-debug", format!("游戏已启动，PID: {}", pid));

    // 发送游戏启动成功的事件到前端
    window.emit("minecraft-launched", format!("游戏已启动，PID: {}", pid))?;

    // 在后台线程中监控游戏进程（带超时）
    spawn_monitor_thread(child, window, pid);

    Ok(())
}

/// 启动监控线程（带超时机制）
fn spawn_monitor_thread(mut child: Child, window: tauri::Window, pid: u32) {
    std::thread::spawn(move || {
        let start_time = Instant::now();
        let is_running = Arc::new(AtomicBool::new(true));

        // 启动超时检查线程
        let is_running_clone = is_running.clone();
        let window_clone = window.clone();
        let timeout_thread = std::thread::spawn(move || {
            while is_running_clone.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_secs(60)); // 每分钟检查一次
                
                if !is_running_clone.load(Ordering::SeqCst) {
                    break;
                }

                let elapsed = start_time.elapsed();
                if elapsed > MAX_GAME_RUNTIME {
                    let _ = window_clone.emit(
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
            Ok(Some(output)) => {
                is_running.store(false, Ordering::SeqCst);
                handle_process_exit(output, &window);
            }
            Ok(None) => {
                // 超时，进程仍在运行
                is_running.store(false, Ordering::SeqCst);
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
                is_running.store(false, Ordering::SeqCst);
                let _ = window.emit("log-error", format!("监控游戏进程时出错: {}", e));
                let _ = window.emit("minecraft-error", format!("监控游戏进程时出错: {}", e));
            }
        }

        // 等待超时检查线程结束
        let _ = timeout_thread.join();
    });
}

/// 等待进程结束（带超时）
fn wait_for_process_with_timeout(
    child: &mut Child,
    timeout: Duration,
) -> Result<Option<std::process::Output>, std::io::Error> {
    let start = Instant::now();

    loop {
        // 检查进程是否已结束
        match child.try_wait()? {
            Some(status) => {
                // 进程已结束，收集输出
                let stdout = child
                    .stdout
                    .take()
                    .map(|mut s| {
                        let mut buf = Vec::new();
                        use std::io::Read;
                        // 使用有限的读取避免阻塞
                        let _ = s.read_to_end(&mut buf);
                        buf
                    })
                    .unwrap_or_default();

                let stderr = child
                    .stderr
                    .take()
                    .map(|mut s| {
                        let mut buf = Vec::new();
                        use std::io::Read;
                        let _ = s.read_to_end(&mut buf);
                        buf
                    })
                    .unwrap_or_default();

                return Ok(Some(std::process::Output {
                    status,
                    stdout,
                    stderr,
                }));
            }
            None => {
                // 进程仍在运行
                if start.elapsed() > timeout {
                    return Ok(None); // 超时
                }
                // 短暂休眠避免 CPU 空转
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    }
}

/// 处理进程退出
fn handle_process_exit(output: std::process::Output, window: &tauri::Window) {
    let status = output.status;

    // 输出 stdout（限制大小避免内存问题）
    if !output.stdout.is_empty() {
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let truncated = if stdout_str.len() > 10000 {
            format!("{}...[truncated]", &stdout_str[..10000])
        } else {
            stdout_str.to_string()
        };
        let _ = window.emit("log-debug", format!("游戏 stdout:\n{}", truncated));
    }

    // 输出 stderr（限制大小）
    if !output.stderr.is_empty() {
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        let truncated = if stderr_str.len() > 10000 {
            format!("{}...[truncated]", &stderr_str[..10000])
        } else {
            stderr_str.to_string()
        };
        let _ = window.emit("log-error", format!("游戏 stderr:\n{}", truncated));
    }

    let _ = window.emit(
        "log-debug",
        format!("游戏进程退出，状态码: {:?}", status.code()),
    );

    // 如果游戏以非零退出码退出，发送错误事件
    if status.code().unwrap_or(-1) != 0 {
        let mut combined = String::new();
        if !output.stdout.is_empty() {
            combined.push_str("[stdout]\n");
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            if stdout_str.len() > 5000 {
                combined.push_str(&stdout_str[..5000]);
                combined.push_str("...[truncated]");
            } else {
                combined.push_str(&stdout_str);
            }
            combined.push('\n');
        }
        if !output.stderr.is_empty() {
            combined.push_str("[stderr]\n");
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            if stderr_str.len() > 5000 {
                combined.push_str(&stderr_str[..5000]);
                combined.push_str("...[truncated]");
            } else {
                combined.push_str(&stderr_str);
            }
        }
        let _ = window.emit(
            "minecraft-error",
            format!(
                "游戏以非零退出 (code={:?})，输出:\n{}",
                status.code(),
                combined
            ),
        );
    }

    // 发送游戏退出事件
    let _ = window.emit(
        "minecraft-exited",
        format!("游戏已退出，状态码: {:?}", status.code()),
    );
}
