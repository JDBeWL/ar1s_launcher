//! 日志初始化模块
//!
//! 用 fern 接管 log 门面，同时输出到 stdout 与 logs 下带时间戳的单个文件

use chrono::Local;
use fern::Dispatch;
use log::LevelFilter;
use std::fs;

pub fn setup_logger() -> Result<(), fern::InitError> {
    // 此时 logger 还没初始化，这里不能用 log 宏
    fs::create_dir_all("logs")?;

    let log_file = format!(
        "logs/ar1s_launcher_{}.log",
        Local::now().format("%Y-%m-%d_%H-%M-%S")
    );

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
