//! 游戏进程启动和监控逻辑

use crate::errors::LauncherError;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
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
    spawn_monitor_thread(child, window, pid);

    Ok(())
}

/// 启动监控线程（带超时机制）
fn spawn_monitor_thread(mut child: Child, window: tauri::Window, pid: u32) {
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let stderr = child.stderr.take().expect("Failed to open stderr");

    let is_running = Arc::new(AtomicBool::new(true));
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
    // 启动 stderr 读取线程
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(l) = line {
                let _ = window_clone_err.emit("log-error", l);
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
                handle_process_exit(status, &window);
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

/// 处理进程退出
fn handle_process_exit(status: ExitStatus, window: &tauri::Window) {
    let _ = window.emit(
        "log-debug",
        format!("游戏进程退出，状态码: {:?}", status.code()),
    );

    // 如果游戏以非零退出码退出，发送错误事件
    if status.code().unwrap_or(-1) != 0 {
        let _ = window.emit(
            "minecraft-error",
            format!("游戏以非零状态码退出: {:?}", status.code()),
        );
    }

    // 发送游戏退出事件
    let _ = window.emit(
        "minecraft-exited",
        format!("游戏已退出，状态码: {:?}", status.code()),
    );
}

