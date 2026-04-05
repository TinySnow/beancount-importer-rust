use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use log::info;

use crate::model::{config::provider::ProviderConfig, mapping::field_mapping::FieldMapping};

use super::{layout::PROVIDER_CATEGORY_DIRS, yaml::load_yaml_file};

/// 加载字段映射配置。
///
/// 规则：
/// - 若供应商显式配置 `mapping_file`，优先使用；
/// - 否则按分层约定路径 `mapping/{banks|securities|third_party}/{provider}.yml` 尝试；
/// - 支持以 `config_base_path` 为基准解析相对路径。
pub(super) fn load_field_mapping(
    provider_config: &ProviderConfig,
    provider_name: &str,
    config_base_path: &Path,
) -> Result<FieldMapping> {
    let mut candidate_paths = Vec::new();

    if let Some(mapping_file) = &provider_config.mapping_file {
        let configured_path = PathBuf::from(mapping_file);
        candidate_paths.extend(resolve_candidate_paths(&configured_path, config_base_path));
    } else {
        for candidate in provider_mapping_fallback_paths(provider_name) {
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
