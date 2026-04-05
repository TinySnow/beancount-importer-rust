use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, Result};
use log::info;

use crate::model::{config::provider::ProviderConfig, mapping::field_mapping::FieldMapping};

use super::{layout::PROVIDER_CATEGORY_DIRS, yaml::load_yaml_file};

/// 加载字段映射配置。
///
/// 规则：
/// - 若供应商显式配置 `mapping_file`，优先使用；
/// - 否则按分层约定路径尝试（并兼容平铺与 `src/mapping` 旧路径）；
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

        // 兼容历史配置：仓库迁移后仍允许旧前缀 `src/mapping/` 自动回退到 `mapping/`。
        for compatibility_path in resolve_mapping_path_compatibility_aliases(&configured_path) {
            candidate_paths.extend(resolve_candidate_paths(
                &compatibility_path,
                config_base_path,
            ));
        }

        // 兼容历史平铺结构：`mapping/<provider>.yml` -> `mapping/<category>/<provider>.yml`。
        for compatibility_path in
            resolve_mapping_flat_to_categorized_aliases(&configured_path, provider_name)
        {
            candidate_paths.extend(resolve_candidate_paths(
                &compatibility_path,
                config_base_path,
            ));
        }
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

/// 生成字段映射候选路径（新结构优先，旧结构兼容）。
///
/// 同时兼容 `mapping/`、`mappings/` 与 `src/mapping/` 三类历史结构。
fn provider_mapping_fallback_paths(provider_name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    for category in PROVIDER_CATEGORY_DIRS {
        paths.push(PathBuf::from(format!(
            "mapping/{}/{}.yml",
            category, provider_name
        )));
    }
    paths.push(PathBuf::from(format!("mapping/{}.yml", provider_name)));

    for category in PROVIDER_CATEGORY_DIRS {
        paths.push(PathBuf::from(format!(
            "mappings/{}/{}.yml",
            category, provider_name
        )));
    }
    paths.push(PathBuf::from(format!("mappings/{}.yml", provider_name)));

    for category in PROVIDER_CATEGORY_DIRS {
        paths.push(PathBuf::from(format!(
            "src/mapping/{}/{}.yml",
            category, provider_name
        )));
    }
    paths.push(PathBuf::from(format!("src/mapping/{}.yml", provider_name)));

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

/// 为 `mapping_file` 生成兼容路径候选。
///
/// 仅处理相对路径，且仅处理“前缀目录迁移”场景：
/// - `src/mapping/*` <-> `mapping/*`
/// - `mappings/*` -> `mapping/*`
///
/// 注意：此函数不访问文件系统，仅生成路径别名。
fn resolve_mapping_path_compatibility_aliases(path: &Path) -> Vec<PathBuf> {
    let mut aliases = Vec::new();

    if let Some(candidate) = replace_relative_prefix(path, &["src", "mapping"], &["mapping"]) {
        aliases.push(candidate.clone());
        if let Some(mappings_candidate) =
            replace_relative_prefix(&candidate, &["mapping"], &["mappings"])
        {
            aliases.push(mappings_candidate);
        }
    }

    if let Some(candidate) = replace_relative_prefix(path, &["mapping"], &["src", "mapping"]) {
        aliases.push(candidate);
    }

    if let Some(candidate) = replace_relative_prefix(path, &["mappings"], &["mapping"]) {
        aliases.push(candidate.clone());
        if let Some(legacy_candidate) =
            replace_relative_prefix(&candidate, &["mapping"], &["src", "mapping"])
        {
            aliases.push(legacy_candidate);
        }
    }

    aliases
}

/// 为“平铺映射路径”生成分层路径候选。
///
/// 仅处理以下历史写法：
/// - `mapping/<provider>.yml`
/// - `mappings/<provider>.yml`
/// - `src/mapping/<provider>.yml`
///
/// 该函数用于将旧的单层路径自动扩展到新分层目录，降低迁移成本。
fn resolve_mapping_flat_to_categorized_aliases(path: &Path, provider_name: &str) -> Vec<PathBuf> {
    if path.is_absolute() {
        return Vec::new();
    }

    let normalized_provider = format!("{}.yml", provider_name.to_ascii_lowercase());

    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => continue,
            Component::Normal(value) => components.push(value.to_string_lossy().to_string()),
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => return Vec::new(),
        }
    }

    let mut aliases = Vec::new();

    let matches_flat_mapping = |left: &str, right: &str| {
        left.eq_ignore_ascii_case(right) || left.eq_ignore_ascii_case(&normalized_provider)
    };

    if components.len() == 2
        && (components[0].eq_ignore_ascii_case("mapping")
            || components[0].eq_ignore_ascii_case("mappings"))
        && matches_flat_mapping(&components[1], &normalized_provider)
    {
        let root = if components[0].eq_ignore_ascii_case("mappings") {
            "mappings"
        } else {
            "mapping"
        };

        for category in PROVIDER_CATEGORY_DIRS {
            aliases.push(PathBuf::from(format!(
                "{}/{}/{}",
                root, category, normalized_provider
            )));
        }
    }

    if components.len() == 3
        && components[0].eq_ignore_ascii_case("src")
        && components[1].eq_ignore_ascii_case("mapping")
        && matches_flat_mapping(&components[2], &normalized_provider)
    {
        for category in PROVIDER_CATEGORY_DIRS {
            aliases.push(PathBuf::from(format!(
                "src/mapping/{}/{}",
                category, normalized_provider
            )));
            aliases.push(PathBuf::from(format!(
                "mapping/{}/{}",
                category, normalized_provider
            )));
        }
    }

    aliases
}

/// 将相对路径前缀替换为新前缀；若不匹配则返回 `None`。
///
/// 仅处理“干净”的相对路径（不含 `..`），以避免目录穿越导致的误判。
fn replace_relative_prefix(
    path: &Path,
    from_prefix: &[&str],
    to_prefix: &[&str],
) -> Option<PathBuf> {
    if path.is_absolute() {
        return None;
    }

    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => continue,
            Component::Normal(value) => components.push(value.to_string_lossy().to_string()),
            // 带 `..` 的复杂相对路径不做目录迁移猜测，避免误判。
            Component::ParentDir => return None,
            Component::Prefix(_) | Component::RootDir => return None,
        }
    }

    if components.len() < from_prefix.len() {
        return None;
    }

    let starts_with_prefix = components
        .iter()
        .take(from_prefix.len())
        .zip(from_prefix.iter())
        .all(|(left, right)| left.eq_ignore_ascii_case(right));

    if !starts_with_prefix {
        return None;
    }

    let mut replaced = PathBuf::new();
    for segment in to_prefix {
        replaced.push(segment);
    }
    for segment in components.iter().skip(from_prefix.len()) {
        replaced.push(segment);
    }

    Some(replaced)
}

/// 按字符串（小写）对路径去重，避免重复 I/O。
///
/// 该去重策略是大小写不敏感的，兼容大小写不敏感文件系统。
fn deduplicate_paths(paths: &mut Vec<PathBuf>) {
    let mut seen = std::collections::HashSet::new();
    paths.retain(|path| seen.insert(path.to_string_lossy().to_ascii_lowercase()));
}
