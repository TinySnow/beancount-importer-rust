//! 运行时配置加载。
//!
//! 负责按优先级加载并聚合三类配置：
//! 1. 全局配置 `GlobalConfig`；
//! 2. 供应商配置 `ProviderConfig`；
//! 3. 字段映射配置 `FieldMapping`。

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use log::{info, warn};
use serde::de::DeserializeOwned;

use crate::model::{
    cli::Cli,
    config::{global::GlobalConfig, provider::ProviderConfig},
    mapping::field_mapping::FieldMapping,
};

/// 运行期完整配置。
pub struct LoadedConfig {
    pub global: GlobalConfig,
    pub provider: ProviderConfig,
    pub mapping: FieldMapping,
}

/// 按约定优先级加载全局/供应商/映射配置。
///
/// 优先级简述：
/// - 显式命令行路径优先；
/// - 约定默认路径其次；
/// - 最后回退到内置默认值（仅全局与供应商）。
pub fn load(cli: &Cli) -> Result<LoadedConfig> {
    let normalized_provider = cli.provider.to_lowercase();

    // 先加载全局配置，后续配置合并需要它。
    let (global_config, global_config_path) = load_global_config(cli.global_config.as_deref())?;

    // 再加载供应商配置，并叠加全局默认字段。
    let (mut provider_config, provider_config_path) =
        load_provider_config(&cli.config, &normalized_provider, &global_config)?;
    provider_config.merge_with_global(&global_config);
    resolve_inventory_seed_paths(
        &mut provider_config,
        provider_config_path
            .as_deref()
            .or(global_config_path.as_deref())
            .or(Some(cli.config.as_path()))
            .unwrap_or_else(|| Path::new(".")),
    );

    // 映射文件相对路径按“供应商/全局/命令行”路径就近解析。
    let mapping_base_path = provider_config_path
        .as_deref()
        .or(global_config_path.as_deref())
        .or(Some(cli.config.as_path()))
        .unwrap_or_else(|| Path::new("."));

    let field_mapping =
        load_field_mapping(&provider_config, &normalized_provider, mapping_base_path)?;

    Ok(LoadedConfig {
        global: global_config,
        provider: provider_config,
        mapping: field_mapping,
    })
}

/// 读取并解析 YAML 文件。
///
/// 额外处理 UTF-8 BOM，避免某些编辑器保存后导致解析失败。
fn load_yaml_file<T>(path: &Path, label: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}: {}", label, path.display()))?;

    // 某些 YAML 文件可能包含 BOM，解析前先剥离。
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);

    serde_yaml::from_str(content)
        .with_context(|| format!("Failed to parse {}: {}", label, path.display()))
}

/// 加载全局配置。
///
/// 当未显式指定路径时，按固定候选路径查找；都找不到则使用默认值。
fn load_global_config(path: Option<&Path>) -> Result<(GlobalConfig, Option<PathBuf>)> {
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

/// 加载供应商配置。
///
/// 顺序：
/// 1. 命令行 `--config` 指定路径；
/// 2. 约定路径 `config/{provider}.yml` 与 `src/config/{provider}.yml`；
/// 3. 全局配置中的 `providers.{provider}` 子配置；
/// 4. 最终回退到默认值。
fn load_provider_config(
    path: &Path,
    provider_name: &str,
    global_config: &GlobalConfig,
) -> Result<(ProviderConfig, Option<PathBuf>)> {
    if path.exists() {
        info!("Loading provider config: {}", path.display());
        let config: ProviderConfig = load_yaml_file(path, "provider config")?;
        return Ok((config, Some(path.to_path_buf())));
    }

    let fallback_paths = [
        PathBuf::from(format!("config/{}.yml", provider_name)),
        PathBuf::from(format!("src/config/{}.yml", provider_name)),
    ];

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

/// 加载字段映射配置。
///
/// 规则：
/// - 若供应商显式配置 `mapping_file`，优先使用；
/// - 否则按约定路径尝试；
/// - 支持以 `config_base_path` 为基准解析相对路径。
fn load_field_mapping(
    provider_config: &ProviderConfig,
    provider_name: &str,
    config_base_path: &Path,
) -> Result<FieldMapping> {
    let mut candidate_paths = Vec::new();

    if let Some(mapping_file) = &provider_config.mapping_file {
        candidate_paths.extend(resolve_candidate_paths(
            &PathBuf::from(mapping_file),
            config_base_path,
        ));
    } else {
        let defaults = [
            PathBuf::from(format!("mapping/{}.yml", provider_name)),
            PathBuf::from(format!("mappings/{}.yml", provider_name)),
            PathBuf::from(format!("src/mapping/{}.yml", provider_name)),
        ];

        for candidate in defaults {
            candidate_paths.extend(resolve_candidate_paths(&candidate, config_base_path));
        }
    }

    // 候选路径大小写不敏感去重，避免重复尝试。
    deduplicate_paths(&mut candidate_paths);

    for path in &candidate_paths {
        if path.exists() {
            info!("Loading field mapping: {}", path.display());
            return load_yaml_file(path, "field mapping");
        }
    }

    let tried_paths = candidate_paths
        .iter()
        .map(|path| format!("- {}", path.display()))
        .collect::<Vec<_>>()
        .join("\n");

    Err(anyhow!(
        "No field mapping file found for provider '{}'. Tried:\n{}",
        provider_name,
        tried_paths
    ))
}

/// 将一个路径扩展为候选列表（绝对路径原样返回，相对路径返回两种解析方式）。
fn resolve_candidate_paths(path: &Path, base_path: &Path) -> Vec<PathBuf> {
    if path.is_absolute() {
        return vec![path.to_path_buf()];
    }

    let base_dir = base_path.parent().unwrap_or_else(|| Path::new("."));
    vec![base_dir.join(path), path.to_path_buf()]
}

/// 按字符串（小写）对路径去重，避免重复 I/O。
fn deduplicate_paths(paths: &mut Vec<PathBuf>) {
    let mut seen = std::collections::HashSet::new();
    paths.retain(|path| seen.insert(path.to_string_lossy().to_ascii_lowercase()));
}

/// Resolves `inventory_seed_files` relative paths against the config file directory.
fn resolve_inventory_seed_paths(provider_config: &mut ProviderConfig, config_base_path: &Path) {
    if provider_config.inventory_seed_files.is_empty() {
        return;
    }

    let base_dir = config_base_path.parent().unwrap_or_else(|| Path::new("."));
    provider_config.inventory_seed_files = provider_config
        .inventory_seed_files
        .iter()
        .map(|raw| {
            let candidate = PathBuf::from(raw);
            if candidate.is_absolute() {
                candidate
            } else {
                base_dir.join(candidate)
            }
        })
        .map(|path| path.to_string_lossy().to_string())
        .collect();
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::model::{
        cli::{Cli, log_level::LogLevel},
        config::{global::GlobalConfig, provider::ProviderConfig},
    };

    use super::{load, load_provider_config, resolve_inventory_seed_paths};

    #[test]
    fn load_provider_config_matches_global_key_case_insensitively() {
        let mut global = GlobalConfig::default();
        let mut provider = ProviderConfig::default();
        provider.default_asset_account = Some("Assets:Broker:Case:Cash".to_string());

        global
            .providers
            .insert("MyProvider".to_string(), provider.clone());

        let (loaded, source_path) =
            load_provider_config(Path::new("__missing__.yml"), "myprovider", &global)
                .expect("global provider context lookup should work");

        assert!(source_path.is_none());
        assert_eq!(loaded.default_asset_account, provider.default_asset_account);
    }

    #[test]
    fn load_normalizes_provider_name_before_resolving_paths() {
        let cli = Cli {
            provider: "WECHAT".to_string(),
            source: PathBuf::from("dummy.csv"),
            config: PathBuf::from("__missing__.yml"),
            global_config: None,
            output: None,
            log_level: LogLevel::Warn,
            quiet: false,
            verbose: false,
            strict: false,
        };

        let loaded =
            load(&cli).expect("uppercase provider name should still resolve config and mapping");
        assert!(loaded.mapping.date.is_some() || loaded.mapping.amount.is_some());
    }

    #[test]
    fn resolves_relative_inventory_seed_paths_against_config_base() {
        let mut provider = ProviderConfig::default();
        provider.inventory_seed_files = vec![
            "transactions/2025/12/galaxy.bean".to_string(),
            "C:/already/absolute.bean".to_string(),
        ];

        resolve_inventory_seed_paths(&mut provider, Path::new("config-new/galaxy.yml"));

        let normalized_first = provider.inventory_seed_files[0].replace('\\', "/");
        assert!(normalized_first.ends_with("config-new/transactions/2025/12/galaxy.bean"));
        assert_eq!(
            provider.inventory_seed_files[1],
            "C:/already/absolute.bean".to_string()
        );
    }
}
