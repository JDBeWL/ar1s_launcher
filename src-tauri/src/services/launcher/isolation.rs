//! 版本隔离
//!
//! Minecraft 只有一个 `--gameDir`，所以只有两种状态：开 = 游戏目录是 `versions/<实例>`，
//! 关 = 游戏目录是根目录，所有实例共用一份 mods / saves / options.txt。
//! 启动参数、工作目录、模组管理、整合包安装必须用同一个判定，否则会出现「列表里有模组、游戏里没有」。

use crate::models::GameConfig;
use crate::services::config;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// 实例内容的落点：模组、存档、配置、日志都在它下面
pub fn content_root(config: &GameConfig, game_dir: &Path, version_id: &str) -> PathBuf {
    content_root_of(
        config,
        game_dir,
        &game_dir.join("versions").join(version_id),
    )
}

pub fn content_root_of(config: &GameConfig, game_dir: &Path, version_dir: &Path) -> PathBuf {
    if config.version_isolation {
        version_dir.to_path_buf()
    } else {
        game_dir.to_path_buf()
    }
}

/// 没有 `GameConfig` 时也走同一判定；配置读不到按默认（隔离）处理，宁可少共享
pub fn content_root_for(game_dir: &Path, version_id: &str) -> PathBuf {
    if config::is_version_isolated() {
        game_dir.join("versions").join(version_id)
    } else {
        game_dir.to_path_buf()
    }
}

/// 启动前备好内容目录：只建 `mods`，saves/logs 游戏会自建，
/// 但 mods 是用户直接往里放文件的地方，缺它模组列表会报错
pub fn prepare_content_root(
    config: &GameConfig,
    game_dir: &Path,
    version_id: &str,
) -> Result<PathBuf, io::Error> {
    let root = content_root(config, game_dir, version_id);
    fs::create_dir_all(root.join("mods"))?;
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::content_root;
    use crate::models::GameConfig;
    use std::path::Path;

    fn config(isolated: bool) -> GameConfig {
        GameConfig {
            version_isolation: isolated,
            ..GameConfig::default()
        }
    }

    #[test]
    fn test_isolation_points_at_the_version_folder() {
        let game = Path::new("C:/game");
        assert_eq!(
            content_root(&config(true), game, "1.20.1-Forge"),
            game.join("versions/1.20.1-Forge")
        );
    }

    #[test]
    fn test_shared_mode_points_at_the_game_root() {
        let game = Path::new("C:/game");
        assert_eq!(content_root(&config(false), game, "1.20.1-Forge"), game);
    }
}
