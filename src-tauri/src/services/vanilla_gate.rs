use crate::errors::LauncherError;

const VANILLA_DOWNLOAD_ENABLED: bool = false;

const DISABLED_MESSAGE: &str = "为遵守相关法律法规，本版本已停止提供 Minecraft 原版游戏文件的下载。";

pub fn is_enabled() -> bool {
    VANILLA_DOWNLOAD_ENABLED
}

pub fn ensure_enabled() -> Result<(), LauncherError> {
    if VANILLA_DOWNLOAD_ENABLED {
        Ok(())
    } else {
        Err(LauncherError::Disabled(DISABLED_MESSAGE.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::{ensure_enabled, is_enabled};
    use crate::errors::LauncherError;

    #[test]
    fn test_switch_state_is_exposed() {
        assert!(!is_enabled());
    }

    #[test]
    fn test_disabled_switch_rejects_with_a_stable_code() {
        let error = ensure_enabled().expect_err("开关关闭时守卫必须拒绝");

        assert_eq!(error.code(), "disabled");
        assert!(matches!(error, LauncherError::Disabled(_)));
        assert!(
            error.to_string().contains("原版"),
            "文案需要说明原因: {}",
            error
        );
    }
}
