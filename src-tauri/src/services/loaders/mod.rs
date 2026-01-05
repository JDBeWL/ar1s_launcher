//! Mod 加载器安装模块
//!
//! 统一管理所有 mod 加载器的安装逻辑：
//! - Forge
//! - Fabric
//! - Quilt
//! - NeoForge

pub mod fabric;
pub mod forge;
pub mod neoforge;
pub mod quilt;

pub use fabric::*;
pub use forge::*;
pub use neoforge::*;
pub use quilt::*;

use crate::errors::LauncherError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 加载器类型枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LoaderType {
    Forge {
        mc_version: String,
        loader_version: String,
    },
    Fabric {
        mc_version: String,
        loader_version: String,
    },
    Quilt {
        mc_version: String,
        loader_version: String,
    },
    NeoForge {
        mc_version: String,
        loader_version: String,
    },
}

impl LoaderType {
    /// 获取 Minecraft 版本
    pub fn mc_version(&self) -> &str {
        match self {
            LoaderType::Forge { mc_version, .. } => mc_version,
            LoaderType::Fabric { mc_version, .. } => mc_version,
            LoaderType::Quilt { mc_version, .. } => mc_version,
            LoaderType::NeoForge { mc_version, .. } => mc_version,
        }
    }

    /// 获取加载器版本
    pub fn loader_version(&self) -> &str {
        match self {
            LoaderType::Forge { loader_version, .. } => loader_version,
            LoaderType::Fabric { loader_version, .. } => loader_version,
            LoaderType::Quilt { loader_version, .. } => loader_version,
            LoaderType::NeoForge { loader_version, .. } => loader_version,
        }
    }

    /// 获取加载器名称
    pub fn name(&self) -> &'static str {
        match self {
            LoaderType::Forge { .. } => "Forge",
            LoaderType::Fabric { .. } => "Fabric",
            LoaderType::Quilt { .. } => "Quilt",
            LoaderType::NeoForge { .. } => "NeoForge",
        }
    }
}

/// 安装加载器的统一入口
pub async fn install_loader(
    loader: &LoaderType,
    instance_name: &str,
    game_dir: &Path,
) -> Result<(), LauncherError> {
    match loader {
        LoaderType::Forge { mc_version, loader_version } => {
            forge::install_forge(mc_version, loader_version, instance_name, game_dir).await
        }
        LoaderType::Fabric { mc_version, loader_version } => {
            fabric::install_fabric(mc_version, loader_version, instance_name, game_dir).await
        }
        LoaderType::Quilt { mc_version, loader_version } => {
            quilt::install_quilt(mc_version, loader_version, instance_name, game_dir).await
        }
        LoaderType::NeoForge { mc_version, loader_version } => {
            neoforge::install_neoforge(mc_version, loader_version, instance_name, game_dir).await
        }
    }
}
