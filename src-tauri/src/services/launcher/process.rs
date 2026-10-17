//! 游戏进程启动和监控逻辑
//!
//! 派生隐藏控制台的 Java 子进程，日志经有界队列批量转发，退出时回显日志尾巴与 hs_err_pid

use crate::errors::LauncherError;
use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tauri::Emitter;

const MAX_GAME_RUNTIME: Duration = Duration::from_secs(24 * 60 * 60);

/// 崩溃/异常退出时回显给前端的最近日志行数（含 stdout）
const MAX_LOG_TAIL_LINES: usize = 100;

/// 队列满时直接丢弃，绝不阻塞读取线程——堵住会让管道写满，游戏日志线程卡住，表现为周期性掉帧
const LOG_QUEUE_CAPACITY: usize = 4096;

const LOG_FLUSH_MAX_LINES: usize = 200;

const LOG_FLUSH_INTERVAL: Duration = Duration::from_millis(200);

const LOG_FLUSH_WAIT: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, PartialEq, Eq)]
enum LogStream {
    Stdout,
    Stderr,
}

type LogItem = (LogStream, String);

/// argv 里 --accessToken 后面跟的是真实的 Minecraft bearer token，进 IPC 之前必须掩掉
fn redacted_args(args: &[String]) -> Vec<&str> {
    let mut out = Vec::with_capacity(args.len());
    let mut mask_next = false;

    for arg in args {
        if mask_next {
            out.push("***");
            mask_next = false;
        } else if arg == "--accessToken" {
            out.push(arg);
            mask_next = true;
        } else if arg.starts_with("--accessToken=") {
            out.push("--accessToken=***");
        } else {
            out.push(arg);
        }
    }

    out
}

pub fn spawn_and_monitor_process(
    java_path: &str,
    final_args: Vec<String>,
    working_dir: &Path,
    window: tauri::Window,
) -> Result<(), LauncherError> {
    let mut command = Command::new(java_path);
    command.args(&final_args);
    command.current_dir(working_dir);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    let display_command = format!("{} {}", java_path, redacted_args(&final_args).join(" "));
    let _ = window.emit("log-debug", format!("最终启动命令: {}", display_command));
    window.emit("launch-command", display_command)?;

    let child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let pid = child.id();
    let _ = window.emit("log-debug", format!("游戏已启动，PID: {}", pid));

    window.emit("minecraft-launched", format!("游戏已启动，PID: {}", pid))?;

    spawn_monitor_thread(child, window, pid, working_dir.to_path_buf());

    Ok(())
}

fn spawn_monitor_thread(
    mut child: Child,
    window: tauri::Window,
    pid: u32,
    working_dir: std::path::PathBuf,
) {
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let stderr = child.stderr.take().expect("Failed to open stderr");

    let is_running = Arc::new(AtomicBool::new(true));
    let stderr_lines: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));

    let (log_tx, log_rx) = mpsc::sync_channel::<LogItem>(LOG_QUEUE_CAPACITY);

    // 读取线程只入队，不做逐行 IPC
    let tx_stdout: SyncSender<LogItem> = log_tx.clone();
    let is_running_stdout = is_running.clone();
    let tail_stdout = stderr_lines.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    push_tail(&tail_stdout, &l);
                    let _ = tx_stdout.try_send((LogStream::Stdout, l));
                }
                Err(_) => break,
            }
            if !is_running_stdout.load(Ordering::SeqCst) {
                break;
            }
        }
    });

    let tx_stderr: SyncSender<LogItem> = log_tx;
    let is_running_stderr = is_running.clone();
    let tail_stderr = stderr_lines.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    push_tail(&tail_stderr, &l);
                    let _ = tx_stderr.try_send((LogStream::Stderr, l));
                }
                Err(_) => break,
            }
            if !is_running_stderr.load(Ordering::SeqCst) {
                break;
            }
        }
    });

    let forwarder_done = Arc::new(AtomicBool::new(false));
    let forwarder_done_flag = forwarder_done.clone();
    let window_forwarder = window.clone();
    let forwarder: JoinHandle<()> = std::thread::spawn(move || {
        forward_game_logs(log_rx, &window_forwarder);
        forwarder_done_flag.store(true, Ordering::SeqCst);
    });

    std::thread::spawn(move || {
        let start_time = Instant::now();
        let is_running_main = is_running.clone();

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

        match wait_for_process_with_timeout(&mut child, MAX_GAME_RUNTIME) {
            Ok(Some(status)) => {
                is_running_main.store(false, Ordering::SeqCst);
                wait_log_flushed(&forwarder_done);
                if forwarder_done.load(Ordering::SeqCst) {
                    let _ = forwarder.join();
                }
                let stderr_buf: Vec<String> = stderr_lines
                    .lock()
                    .map(|buf| buf.iter().cloned().collect())
                    .unwrap_or_default();
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
                    format!(
                        "游戏运行超过 {} 小时，监控已停止",
                        MAX_GAME_RUNTIME.as_secs() / 3600
                    ),
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

/// 追加一行到环形缓冲（保留最近 MAX_LOG_TAIL_LINES 行）
fn push_tail(tail: &Mutex<VecDeque<String>>, line: &str) {
    if let Ok(mut buf) = tail.lock() {
        if buf.len() >= MAX_LOG_TAIL_LINES {
            buf.pop_front();
        }
        buf.push_back(line.to_string());
    }
}

fn wait_log_flushed(forwarder_done: &AtomicBool) {
    let deadline = Instant::now() + LOG_FLUSH_WAIT;
    while !forwarder_done.load(Ordering::SeqCst) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// 按固定节奏批量转发日志到前端，避免逐行事件淹没 WebView
fn forward_game_logs(rx: Receiver<LogItem>, window: &tauri::Window) {
    let mut batch: Vec<LogItem> = Vec::with_capacity(LOG_FLUSH_MAX_LINES);

    loop {
        match rx.recv_timeout(LOG_FLUSH_INTERVAL) {
            Ok(item) => {
                batch.push(item);
                while batch.len() < LOG_FLUSH_MAX_LINES {
                    match rx.try_recv() {
                        Ok(item) => batch.push(item),
                        Err(_) => break,
                    }
                }
                flush_log_batch(&batch, window);
                batch.clear();
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    flush_log_batch(&batch, window);
}

fn flush_log_batch(batch: &[LogItem], window: &tauri::Window) {
    if batch.is_empty() {
        return;
    }

    let mut out = String::new();
    let mut err = String::new();
    for (stream, line) in batch {
        let target = match stream {
            LogStream::Stdout => &mut out,
            LogStream::Stderr => &mut err,
        };
        target.push_str(line);
        target.push('\n');
    }

    if !out.is_empty() {
        let _ = window.emit("log-debug", out);
    }
    if !err.is_empty() {
        let _ = window.emit("log-error", err);
    }
}

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

    if exit_code.unwrap_or(-1) != 0 {
        let mut error_msg = format!("游戏以非零状态码退出: {:?}", exit_code);

        if !stderr_lines.is_empty() {
            error_msg.push_str("\n\n--- 游戏错误输出 ---\n");
            for line in stderr_lines {
                error_msg.push_str(line);
                error_msg.push('\n');
            }
        }

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

    let _ = window.emit(
        "minecraft-exited",
        format!("游戏已退出，状态码: {:?}", exit_code),
    );
}

#[cfg(test)]
mod tests {
    use super::redacted_args;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_redacted_args_masks_token_after_flag() {
        let input = args(&[
            "--username",
            "Steve",
            "--accessToken",
            "SECRET",
            "--version",
            "1.20.4",
        ]);
        assert_eq!(
            redacted_args(&input),
            [
                "--username",
                "Steve",
                "--accessToken",
                "***",
                "--version",
                "1.20.4"
            ]
        );
    }

    #[test]
    fn test_redacted_args_masks_inline_token() {
        let input = args(&["--accessToken=SECRET", "--gameDir", "dir"]);
        assert_eq!(
            redacted_args(&input),
            ["--accessToken=***", "--gameDir", "dir"]
        );
        assert!(redacted_args(&Vec::new()).is_empty());
    }
}
