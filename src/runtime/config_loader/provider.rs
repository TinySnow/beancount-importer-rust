use std::path::{Path, PathBuf};

use anyhow::Result;
use log::{info, warn};

use crate::model::config::{global::GlobalConfig, provider::ProviderConfig};

use super::{layout::PROVIDER_CATEGORY_DIRS, yaml::load_yaml_file};

/// 加载供应商配置。
///
/// 顺序：
/// 1. 命令行 `--config` 指定路径；
/// 2. 分层约定路径（`config/{banks|securities|third_party}/{provider}.yml`）；
/// 3. 兼容平铺路径 `config/{provider}.yml` 与 `src/config/{provider}.yml`；
/// 4. 全局配置中的 `providers.{provider}` 子配置；
/// 5. 最终回退到默认值。
pub(super) fn load_provider_config(
    path: &Path,
    provider_name: &str,
    global_config: &GlobalConfig,
) -> Result<(ProviderConfig, Option<PathBuf>)> {
    if path.exists() {
        info!("Loading provider config: {}", path.display());
        let config: ProviderConfig = load_yaml_file(path, "provider config")?;
        return Ok((config, Some(path.to_path_buf())));
    }

    let fallback_paths = provider_config_fallback_paths(provider_name);

    for fallback in fallback_paths {
        if fallback.exists() {
            info!(
                "Provider config '{}' not found, using fallback: {}",
                path.display(),
                fallback.display()
            );
            let config: ProviderConfig = load_yaml_file(&fallback, "provider config")?;
            return Ok((config, Some(fallback)));
        }
    }

    if let Some((provider_key, provider_config)) = global_config
        .providers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(provider_name))
    {
        info!(
            "Using provider config for '{}' from global config context key '{}'",
            provider_name, provider_key
        );
        return Ok((provider_config.clone(), None));
    }

    warn!(
        "Provider config not found for '{}', using defaults",
        provider_name
    );
    Ok((ProviderConfig::default(), None))
}

/// 生成 provider 配置候选路径（新结构优先，旧结构兼容）。
///
/// 结果顺序反映查找优先级：先 `config/` 分层，再平铺，再 `src/config/` 兼容路径。
fn provider_config_fallback_paths(provider_name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    for category in PROVIDER_CATEGORY_DIRS {
        paths.push(PathBuf::from(format!(
            "config/{}/{}.yml",
            category, provider_name
        )));
    }
    paths.push(PathBuf::from(format!("config/{}.yml", provider_name)));

    for category in PROVIDER_CATEGORY_DIRS {
        paths.push(PathBuf::from(format!(
            "src/config/{}/{}.yml",
            category, provider_name
        )));
    }
    paths.push(PathBuf::from(format!("src/config/{}.yml", provider_name)));

    paths
}
