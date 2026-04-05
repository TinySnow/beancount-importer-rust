use std::path::{Path, PathBuf};

use anyhow::Result;
use log::{info, warn};

use crate::model::config::{global::GlobalConfig, provider::ProviderConfig};

use super::{layout::PROVIDER_CATEGORY_DIRS, yaml::load_yaml_file};

/// 加载供应商配置。
///
/// 顺序：
/// 1. 命令行 `--config` 指定路径；
/// 2. 分层约定路径 `config/{banks|securities|third_party}/{provider}.yml`；
/// 3. 全局配置中的 `providers.{provider}` 子配置；
/// 4. 最终回退到默认值。
pub(super) fn load_provider_config(
    path: &Path,
    provider_name: &str,
    global_config: &GlobalConfig,
) -> Result<(ProviderConfig, Option<PathBuf>)> {
    if path.exists() {
        info!("Loading provider config: {}", path.display());
        let mut config: ProviderConfig = load_yaml_file(path, "provider config")?;
        config.normalize_default_group();
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
            let mut config: ProviderConfig = load_yaml_file(&fallback, "provider config")?;
            config.normalize_default_group();
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
        let mut provider_config = provider_config.clone();
        provider_config.normalize_default_group();
        return Ok((provider_config, None));
    }

    warn!(
        "Provider config not found for '{}', using defaults",
        provider_name
    );
    Ok((ProviderConfig::default(), None))
}

/// 生成 provider 配置候选路径。
///
/// 结果顺序反映查找优先级：`config/` 分层目录按类别依次尝试。
fn provider_config_fallback_paths(provider_name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    for category in PROVIDER_CATEGORY_DIRS {
        paths.push(PathBuf::from(format!(
            "config/{}/{}.yml",
            category, provider_name
        )));
    }

    paths
}
