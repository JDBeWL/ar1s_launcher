use chrono::Local;
use fern::Dispatch;
use log::LevelFilter;
use std::fs;

pub fn setup_logger() -> Result<(), fern::InitError> {
    // 创建日志目录
    log::info!("[DEBUG] 创建日志目录");
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
