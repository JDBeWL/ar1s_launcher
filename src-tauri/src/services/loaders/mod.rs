//! Mod 加载器安装模块
//!
//! 安装四种加载器并生成带 inheritsFrom 的实例版本 JSON，LoaderType 负责路由

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
    pub fn mc_version(&self) -> &str {
        match self {
            LoaderType::Forge { mc_version, .. } => mc_version,
            LoaderType::Fabric { mc_version, .. } => mc_version,
            LoaderType::Quilt { mc_version, .. } => mc_version,
            LoaderType::NeoForge { mc_version, .. } => mc_version,
        }
    }

    pub fn loader_version(&self) -> &str {
        match self {
            LoaderType::Forge { loader_version, .. } => loader_version,
            LoaderType::Fabric { loader_version, .. } => loader_version,
            LoaderType::Quilt { loader_version, .. } => loader_version,
            LoaderType::NeoForge { loader_version, .. } => loader_version,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            LoaderType::Forge { .. } => "Forge",
            LoaderType::Fabric { .. } => "Fabric",
            LoaderType::Quilt { .. } => "Quilt",
            LoaderType::NeoForge { .. } => "NeoForge",
        }
    }
}

pub async fn install_loader(
    loader: &LoaderType,
    instance_name: &str,
    game_dir: &Path,
) -> Result<(), LauncherError> {
    match loader {
        LoaderType::Forge {
            mc_version,
            loader_version,
        } => forge::install_forge(mc_version, loader_version, instance_name, game_dir).await,
        LoaderType::Fabric {
            mc_version,
            loader_version,
        } => fabric::install_fabric(mc_version, loader_version, instance_name, game_dir).await,
        LoaderType::Quilt {
            mc_version,
            loader_version,
        } => quilt::install_quilt(mc_version, loader_version, instance_name, game_dir).await,
        LoaderType::NeoForge {
            mc_version,
            loader_version,
        } => neoforge::install_neoforge(mc_version, loader_version, instance_name, game_dir).await,
    }
}
