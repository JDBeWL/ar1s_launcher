//! 版本隔离目录准备

use crate::models::GameConfig;
use std::fs;
use std::io;
use std::path::Path;

/// 准备版本隔离目录
pub fn prepare_isolated_version_directory(
    config: &GameConfig,
    game_dir: &Path,
    version_dir: &Path,
) -> Result<(), io::Error> {
    if !config.version_isolation {
        return Ok(());
    }

    let isolate_dirs = [
        ("saves", config.isolate_saves),
        ("resourcepacks", config.isolate_resourcepacks),
        ("logs", config.isolate_logs),
    ];

    for (dir_name, should_isolate) in isolate_dirs {
        let dir_path = version_dir.join(dir_name);
        if should_isolate && !dir_path.exists() {
            fs::create_dir_all(&dir_path)?;
        }
    }

    // 复制 options.txt
    let options_src = game_dir.join("options.txt");
    let options_dst = version_dir.join("options.txt");
    if options_src.exists() && !options_dst.exists() {
        fs::copy(&options_src, &options_dst)?;
    }

    Ok(())
}
