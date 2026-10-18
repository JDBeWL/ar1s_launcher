//! 日志初始化模块
//!
//! 用 fern 接管 log 门面，同时输出到 stdout 与 logs 下带时间戳的单个文件

use chrono::Local;
use fern::Dispatch;
use log::LevelFilter;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const MAX_LOG_FILES: usize = 30;

pub fn setup_logger(log_dir: &Path) -> Result<(), fern::InitError> {
    // 此时 logger 还没初始化，这里不能用 log 宏
    fs::create_dir_all(log_dir)?;
    prune_old_logs(log_dir, MAX_LOG_FILES.saturating_sub(1));

    let log_file = log_dir.join(format!(
        "ar1s_launcher_{}.log",
        Local::now().format("%Y-%m-%d_%H-%M-%S")
    ));

    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                message
            ))
        })
        .level(LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file(&log_file)?)
        .apply()?;

    Ok(())
}

/// 只保留最近的若干个 .log，避免每次启动都新增一个永不清理的文件
fn prune_old_logs(log_dir: &Path, keep: usize) {
    let Ok(entries) = fs::read_dir(log_dir) else {
        return;
    };

    let mut logs: Vec<(SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with("ar1s_")
                && entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("log"))
        })
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, entry.path()))
        })
        .collect();

    if logs.len() <= keep {
        return;
    }

    logs.sort_by_key(|(modified, _)| *modified);
    let remove_count = logs.len() - keep;
    for (_, path) in logs.into_iter().take(remove_count) {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::prune_old_logs;
    use std::fs;
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_prune_old_logs_keeps_newest_and_ignores_other_files() {
        let dir = tempfile::TempDir::new().unwrap();
        for i in 0..5 {
            let path = dir.path().join(format!("ar1s_{i}.log"));
            fs::write(&path, b"x").unwrap();
            let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
            file.set_modified(SystemTime::now() - Duration::from_secs(100 - i))
                .unwrap();
        }
        fs::write(dir.path().join("keep.txt"), b"x").unwrap();
        let other_log = dir.path().join("other.log");
        fs::write(&other_log, b"x").unwrap();
        let file = fs::OpenOptions::new().write(true).open(&other_log).unwrap();
        file.set_modified(SystemTime::now() - Duration::from_secs(200))
            .unwrap();

        prune_old_logs(dir.path(), 3);

        let remaining = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("ar1s_"))
            .count();
        assert_eq!(remaining, 3);
        assert!(dir.path().join("ar1s_4.log").exists());
        assert!(!dir.path().join("ar1s_0.log").exists());
        assert!(dir.path().join("keep.txt").exists());
        assert!(other_log.exists(), "不能删除其他程序的日志");
    }
}
