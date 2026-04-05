use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use log::{info, warn};

use crate::model::config::global::GlobalConfig;

use super::yaml::load_yaml_file;

/// 加载全局配置。
///
/// 当未显式指定路径时，按固定候选路径查找；找不到则使用默认值。
pub(super) fn load_global_config(path: Option<&Path>) -> Result<(GlobalConfig, Option<PathBuf>)> {
    if let Some(path) = path {
        if !path.exists() {
            return Err(anyhow!("Global config path not found: {}", path.display()));
        }

        info!("Loading global config: {}", path.display());
        let mut config: GlobalConfig = load_yaml_file(path, "global config")?;
        config.normalize_default_group();
        return Ok((config, Some(path.to_path_buf())));
    }

    let fallback_path = PathBuf::from("config/global.yml");
    if fallback_path.exists() {
        info!("Loading global config: {}", fallback_path.display());
        let mut config: GlobalConfig = load_yaml_file(&fallback_path, "global config")?;
        config.normalize_default_group();
        return Ok((config, Some(fallback_path)));
    }

    warn!("Global config file not found, using built-in defaults");
    Ok((GlobalConfig::default(), None))
}
