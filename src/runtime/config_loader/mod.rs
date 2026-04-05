//! 运行时配置加载模块。
//!
//! 文件路径：`src/runtime/config_loader/mod.rs`。
//!
//! 该模块负责在运行时加载并组装三类核心配置：
//! - 全局配置（[`GlobalConfig`]）
//! - 供应商配置（[`ProviderConfig`]）
//! - 字段映射（[`FieldMapping`]）
//!
//! 说明：`mod.rs` 仅保留高层编排，具体加载/路径兼容逻辑拆分到子模块。

mod global;
mod inventory;
mod layout;
mod mapping;
mod provider;
mod yaml;

use std::path::Path;

use anyhow::Result;

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
pub fn load(cli: &Cli) -> Result<LoadedConfig> {
    let normalized_provider = cli.provider.to_lowercase();

    // 先加载全局配置，后续配置合并需要它。
    let (global_config, global_config_path) =
        global::load_global_config(cli.global_config.as_deref())?;

    // 再加载供应商配置，并叠加全局默认字段。
    let (mut provider_config, provider_config_path) =
        provider::load_provider_config(&cli.config, &normalized_provider, &global_config)?;
    provider_config.merge_with_global(&global_config);

    inventory::resolve_inventory_seed_paths(
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
        mapping::load_field_mapping(&provider_config, &normalized_provider, mapping_base_path)?;

    Ok(LoadedConfig {
        global: global_config,
        provider: provider_config,
        mapping: field_mapping,
    })
}

#[cfg(test)]
use inventory::resolve_inventory_seed_paths;
#[cfg(test)]
use mapping::load_field_mapping;
#[cfg(test)]
use provider::load_provider_config;

#[cfg(test)]
mod tests;
