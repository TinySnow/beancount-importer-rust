use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use log::info;

use crate::model::{config::provider::ProviderConfig, mapping::field_mapping::FieldMapping};

use super::{layout::PROVIDER_CATEGORY_DIRS, yaml::load_yaml_file};

include!(concat!(env!("OUT_DIR"), "/embedded_mappings.rs"));

/// 加载字段映射配置。
///
/// 优先级（从高到低）：
/// 1. CLI `--mapping` 显式指定；
/// 2. provider 配置中的 `mapping_file` 字段；
/// 3. 分层约定路径 `mapping/{banks|securities|third_party}/{provider}.yml`；
/// 4. 编译期内嵌 mapping（仅内置供应商）。
///
/// 内嵌 mapping 始终作为最终回退，不受 `mapping_file` 是否设置影响。
pub(super) fn load_field_mapping(
    cli_mapping: Option<&Path>,
    provider_config: &ProviderConfig,
    provider_name: &str,
    config_base_path: &Path,
) -> Result<FieldMapping> {
    // 1. CLI --mapping 最高优先级
    if let Some(cli_path) = cli_mapping {
        info!("Loading field mapping from CLI: {}", cli_path.display());
        return load_yaml_file(cli_path, "field mapping");
    }

    // 2. 收集候选路径：provider 显式 mapping_file + 分层约定路径
    let mut candidate_paths = Vec::new();

    if let Some(mapping_file) = &provider_config.mapping_file {
        let configured_path = PathBuf::from(mapping_file);
        candidate_paths.extend(resolve_candidate_paths(&configured_path, config_base_path));
    }

    for candidate in provider_mapping_fallback_paths(provider_name) {
        candidate_paths.extend(resolve_candidate_paths(&candidate, config_base_path));
    }

    // 候选路径大小写不敏感去重，避免重复尝试。
    deduplicate_paths(&mut candidate_paths);

    // 3. 尝试文件系统路径
    for path in &candidate_paths {
        if path.exists() {
            info!("Loading field mapping: {}", path.display());
            return load_yaml_file(path, "field mapping");
        }
    }

    // 4. 内嵌 mapping 始终作为最终回退
    if let Some(mapping) = load_embedded_field_mapping(provider_name)? {
        info!(
            "Loading embedded field mapping for provider '{}'",
            provider_name
        );
        return Ok(mapping);
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

/// 生成字段映射候选路径。
///
/// 结果顺序反映查找优先级：`mapping/` 分层目录按类别依次尝试。
fn provider_mapping_fallback_paths(provider_name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    for category in PROVIDER_CATEGORY_DIRS {
        paths.push(PathBuf::from(format!(
            "mapping/{}/{}.yml",
            category, provider_name
        )));
    }

    paths
}

/// 将一个路径扩展为候选列表（绝对路径原样返回，相对路径返回两种解析方式）。
///
/// 相对路径会生成：
/// 1. 相对 `base_path` 所在目录的路径；
/// 2. 相对当前工作目录的原路径。
fn resolve_candidate_paths(path: &Path, base_path: &Path) -> Vec<PathBuf> {
    if path.is_absolute() {
        return vec![path.to_path_buf()];
    }

    let base_dir = base_path.parent().unwrap_or_else(|| Path::new("."));
    vec![base_dir.join(path), path.to_path_buf()]
}

/// 按字符串（小写）对路径去重，避免重复 I/O。
///
/// 该去重策略是大小写不敏感的，兼容大小写不敏感文件系统。
fn deduplicate_paths(paths: &mut Vec<PathBuf>) {
    let mut seen = std::collections::HashSet::new();
    paths.retain(|path| seen.insert(path.to_string_lossy().to_ascii_lowercase()));
}

/// 加载内嵌字段映射（仅内置供应商）。
pub(super) fn load_embedded_field_mapping(provider_name: &str) -> Result<Option<FieldMapping>> {
    let Some(yaml) = embedded_mapping_yaml(provider_name) else {
        return Ok(None);
    };

    let yaml = yaml.strip_prefix('\u{feff}').unwrap_or(yaml);
    let mapping = serde_yaml::from_str::<FieldMapping>(yaml).with_context(|| {
        format!(
            "Failed to parse embedded field mapping for provider '{}'",
            provider_name
        )
    })?;

    Ok(Some(mapping))
}

fn embedded_mapping_yaml(provider_name: &str) -> Option<&'static str> {
    let provider_name = provider_name.to_ascii_lowercase();
    EMBEDDED_FIELD_MAPPINGS
        .iter()
        .find_map(|(provider, yaml)| (*provider == provider_name).then_some(*yaml))
}

#[cfg(test)]
pub(super) fn embedded_mapping_provider_names() -> Vec<&'static str> {
    EMBEDDED_FIELD_MAPPINGS
        .iter()
        .map(|(provider, _)| *provider)
        .collect()
}
