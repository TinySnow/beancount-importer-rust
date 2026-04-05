//! 运行时配置加载模块。
//!
//! 文件路径：`src/runtime/config_loader/mod.rs`。
//!
//! 该模块负责在运行时加载并组装三类核心配置：
//! - 全局配置（[`GlobalConfig`]）
//! - 供应商配置（[`ProviderConfig`]）
//! - 字段映射（[`FieldMapping`]）
//!
//! 同时还负责以下兼容与路径处理逻辑：
//! - 兼容历史目录结构（`src/config`、`src/mapping`、平铺路径）。
//! - 基于配置文件位置解析相对路径（如 `mapping_file`、`inventory_seed_files`）。
//! - 在缺省配置场景回退到内置默认值。
//!
//! 说明：该模块属于 `runtime` 私有子模块，对外统一由 `runtime::run` 调用。

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use log::{info, warn};
use serde::de::DeserializeOwned;

use crate::model::{
    cli::Cli,
    config::{global::GlobalConfig, provider::ProviderConfig},
    mapping::field_mapping::FieldMapping,
};

/// 加载完成后的运行时配置快照。
///
/// 该结构是配置加载阶段的输出，会被后续流水线直接消费。
pub struct LoadedConfig {
    /// 全局配置（共享默认项、全局规则、可选供应商上下文）。
    pub global: GlobalConfig,
    /// 供应商配置（已合并全局默认项，并完成路径归一化）。
    pub provider: ProviderConfig,
    /// 字段映射配置（用于源数据列到标准字段的映射）。
    pub mapping: FieldMapping,
}

/// 按约定优先级加载全局/供应商/映射配置。
///
/// 加载顺序与优先级：
/// - 显式命令行路径优先；
/// - 约定默认路径其次；
/// - 最后回退到内置默认值（仅全局与供应商）。
///
/// # 参数
/// - `cli`：命令行参数，包含供应商名、配置路径、输出路径等上下文。
///
/// # 返回值
/// - `Ok(LoadedConfig)`：配置全部解析成功。
/// - `Err(anyhow::Error)`：任一步骤读取或解析失败。
///
/// # 错误
/// 当全局配置/供应商配置/字段映射读取或解析失败时返回错误。
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

/// 读取并解析一个 YAML 文件。
///
/// 额外处理 UTF-8 BOM，避免某些编辑器保存后导致解析失败。
///
/// # 类型参数
/// - `T`：目标反序列化类型。
///
/// # 参数
/// - `path`：YAML 文件路径。
/// - `label`：用于错误上下文的逻辑名称。
///
/// # 返回值
/// - `Ok(T)`：反序列化成功。
/// - `Err(anyhow::Error)`：读取失败或 YAML 解析失败。
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
/// 当前优先 `config/`，并保留 `src/config/` 兼容回退。
///
/// # 参数
/// - `path`：可选的显式全局配置路径（通常来自 `--global-config`）。
///
/// # 返回值
/// - `GlobalConfig`：最终生效的全局配置。
/// - `Option<PathBuf>`：配置来源路径。若走默认值回退则为 `None`。
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
/// 2. 分层约定路径（`config/{banks|securities|third_party}/{provider}.yml`）；
/// 3. 兼容平铺路径 `config/{provider}.yml` 与 `src/config/{provider}.yml`；
/// 4. 全局配置中的 `providers.{provider}` 子配置；
/// 5. 最终回退到默认值。
///
/// # 参数
/// - `path`：命令行传入的供应商配置路径（`--config`）。
/// - `provider_name`：已归一化（小写）的供应商标识。
/// - `global_config`：全局配置，用于 provider 上下文回退。
///
/// # 返回值
/// - `ProviderConfig`：最终生效的供应商配置。
/// - `Option<PathBuf>`：配置来源路径。若来自 `global.providers` 或默认值则为 `None`。
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

/// 加载字段映射配置。
///
/// 规则：
/// - 若供应商显式配置 `mapping_file`，优先使用；
/// - 否则按分层约定路径尝试（并兼容平铺与 `src/mapping` 旧路径）；
/// - 支持以 `config_base_path` 为基准解析相对路径。
///
/// # 参数
/// - `provider_config`：供应商配置（可能含 `mapping_file`）。
/// - `provider_name`：供应商名称（用于生成回退候选）。
/// - `config_base_path`：用于解析相对路径的基准配置路径。
///
/// # 返回值
/// - `Ok(FieldMapping)`：找到并成功解析映射文件。
/// - `Err(anyhow::Error)`：候选路径全部失败时返回并附带尝试列表。
fn load_field_mapping(
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

/// 供应商配置/映射分层目录名（按尝试顺序）。
const PROVIDER_CATEGORY_DIRS: [&str; 3] = ["third_party", "banks", "securities"];

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

/// 判断字符串路径是否应视为“已绝对化路径”（跨平台）。
///
/// 除了 `Path::is_absolute()`，还兼容：
/// - Windows 盘符路径：`C:/...`、`C:\...`
/// - Windows UNC 路径：`\\server\share\...`
fn is_effectively_absolute_path(raw: &str) -> bool {
    let candidate = Path::new(raw);
    if candidate.is_absolute() {
        return true;
    }

    let bytes = raw.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\')
    {
        return true;
    }

    raw.starts_with("\\\\")
}

/// 将 `inventory_seed_files` 的相对路径解析为相对配置文件目录的绝对路径。
///
/// 绝对路径（含 Windows 盘符/UNC）保持不变；相对路径会被重写为
/// `config_base_path` 所在目录下的路径字符串。
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
            if is_effectively_absolute_path(raw) {
                candidate
            } else {
                base_dir.join(candidate)
            }
        })
        .map(|path| path.to_string_lossy().to_string())
        .collect();
}

#[cfg(test)]
mod tests;
