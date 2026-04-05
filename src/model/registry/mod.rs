//! Provider 注册表模型。
//!
//! 本模块用于管理导入供应商（[`Provider`](crate::interface::provider::Provider)）的
//! 注册、查找与枚举能力，供运行时按名称选择具体适配器。
//!
//! # 主要类型
//! - [`provider_registry::ProviderRegistry`]：供应商注册表。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::registry::provider_registry::ProviderRegistry;
//!
//! let registry = ProviderRegistry::new();
//! assert!(registry.list_providers().is_empty());
//! ```

pub mod provider_registry;
