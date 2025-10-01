use std::fs;
use std::path::PathBuf;
use crate::error::LauncherError;
use crate::models::GameConfig;

/// 加载配置文件
pub fn load_config() -> Result<GameConfig, LauncherError> {
    let config_path = get_config_path()?;
    
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        let config: GameConfig = serde_json::from_str(&content)?;
        Ok(config)
    } else {
        // 获取可执行文件路径并确保其存在
        let exe_path = std::env::current_exe()?;
        let exe_dir = exe_path.parent()
            .ok_or_else(|| LauncherError::Custom("无法获取可执行文件目录".to_string()))?;
        
        // 创建路径变量并确保所有权
        let mc_dir = exe_dir.join(".minecraft");
        let mc_dir_str = mc_dir.to_string_lossy().into_owned();
        
        // 创建目录结构
        if !mc_dir.exists() {
            fs::create_dir_all(&mc_dir)?;
            // 创建必要的子目录
            let sub_dirs = ["versions", "libraries", "assets", "saves", "resourcepacks", "logs"];
            for dir in sub_dirs {
                fs::create_dir_all(mc_dir.join(dir))?;
            }
        }

        // 创建并返回配置
        let config = GameConfig {
            game_dir: mc_dir_str,
            version_isolation: true,
            java_path: None,
            download_threads: 8,
            language: Some("zh_cn".to_string()),
            isolate_saves: true,
            isolate_resourcepacks: true,
            isolate_logs: true,
            username: None,
            uuid: None,
            max_memory: crate::models::default_max_memory(),
        };
        
        // 保存配置
        save_config(&config)?;

        Ok(config)
    }
}

/// 保存配置文件
pub fn save_config(config: &GameConfig) -> Result<(), LauncherError> {
    let config_path = get_config_path()?;
    fs::write(config_path, serde_json::to_string_pretty(config)?)?;
    Ok(())
}

/// 获取配置文件路径
fn get_config_path() -> Result<PathBuf, LauncherError> {
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent()
        .ok_or_else(|| LauncherError::Custom("无法获取可执行文件目录".to_string()))?;
    
    Ok(exe_dir.join("ar1s.json"))
}