use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use log::{info, warn};

use crate::model::config::global::GlobalConfig;

use super::yaml::load_yaml_file;

/// 加载全局配置。
///
/// 当未显式指定路径时，按固定候选路径查找；都找不到则使用默认值。
/// 当前优先 `config/`，并保留 `src/config/` 兼容回退。
pub(super) fn load_global_config(path: Option<&Path>) -> Result<(GlobalConfig, Option<PathBuf>)> {
    if let Some(path) = path {
        if !path.exists() {
            return Err(anyhow!("Global config path not found: {}", path.display()));
        }

        info!("Loading global config: {}", path.display());
        let config = load_yaml_file(path, "global config")?;
        return Ok((config, Some(path.to_path_buf())));
    }

    let fallback_paths = [
        PathBuf::from("config/global.yml"),
        PathBuf::from("src/config/global.yml"),
    ];

    for path in fallback_paths {
        if path.exists() {
            info!("Loading global config: {}", path.display());
            let config = load_yaml_file(&path, "global config")?;
            return Ok((config, Some(path)));
        }
    }

    warn!("Global config file not found, using built-in defaults");
    Ok((GlobalConfig::default(), None))
}
