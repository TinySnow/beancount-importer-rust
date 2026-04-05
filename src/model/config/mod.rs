//! 配置模型总入口。
//!
//! 该模块集中声明导入流程使用的配置数据结构，涵盖：
//! - 全局默认配置（[`global::GlobalConfig`]）
//! - 单供应商配置（[`provider::ProviderConfig`]）
//! - 通用默认字段分组（[`defaults::CommonDefaultsConfig`]）
//! - 输出格式配置（[`output::OutputConfig`]）
//! - 表格解析选项（[`tabular_options::TabularOptions`]）
//! - 元数据值类型（[`meta_value::MetaValue`]）
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::config::global::GlobalConfig;
//! use beancount_importer_rust::model::config::provider::ProviderConfig;
//!
//! let global = GlobalConfig::default();
//! let mut provider = ProviderConfig::default();
//!
//! provider.merge_with_global(&global);
//! assert_eq!(provider.default_currency.as_deref(), Some("CNY"));
//! ```

pub mod defaults;
pub mod global;
pub mod meta_value;
pub mod output;
pub mod provider;
pub mod tabular_options;
