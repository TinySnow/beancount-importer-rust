//! 供应商注册表实现。
//!
//! 该模块提供 [`ProviderRegistry`]，用于在运行时维护 `provider_name -> provider_impl`
//! 的映射关系，并支持：
//! - 注册供应商；
//! - 按名称（大小写不敏感）查找供应商；
//! - 按稳定顺序列出已注册供应商名称（便于 CLI 提示和测试断言）。
//!
//! # 设计约束
//! - 注册表内部统一将键转换为小写，避免命令行参数大小写差异导致查找失败。
//! - 供应商实例通过 `Arc<dyn Provider>` 存储，允许在多处共享同一实现实例。
//!
//! # 示例
//! ```rust
//! use std::sync::Arc;
//!
//! use beancount_importer_rust::{
//!     error::ImporterResult,
//!     interface::provider::Provider,
//!     model::{
//!         config::provider::ProviderConfig,
//!         data::raw_record::RawRecord,
//!         registry::provider_registry::ProviderRegistry,
//!         rule::rule_engine::RuleEngine,
//!         transaction::Transaction,
//!     },
//! };
//!
//! struct DemoProvider;
//!
//! impl Provider for DemoProvider {
//!     fn name(&self) -> &'static str {
//!         "demo"
//!     }
//!
//!     fn transform(
//!         &self,
//!         _record: RawRecord,
//!         _rule_engine: &RuleEngine,
//!         _config: &ProviderConfig,
//!     ) -> ImporterResult<Option<Transaction>> {
//!         Ok(None)
//!     }
//! }
//!
//! let mut registry = ProviderRegistry::new();
//! registry.register(Arc::new(DemoProvider));
//!
//! let provider = registry.get("DeMo").expect("provider should exist");
//! assert_eq!(provider.name(), "demo");
//! assert_eq!(registry.list_providers(), vec!["demo"]);
//! ```

use std::{collections::HashMap, sync::Arc};

use crate::{interface::provider::Provider, providers::GLOBAL_REGISTRY};

/// 运行时供应商注册表。
///
/// 该结构负责保存可用供应商实例，并提供统一的查找入口。
/// 内部键一律为小写，外部可使用任意大小写进行检索。
pub struct ProviderRegistry {
    /// 供应商索引，键为 `provider.name().to_lowercase()`。
    ///
    /// 使用 `Arc` 可以让调用方按需克隆引用，避免复制底层 provider 实例。
    providers: HashMap<String, Arc<dyn Provider>>,
}

impl ProviderRegistry {
    /// 创建一个空的供应商注册表。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::registry::provider_registry::ProviderRegistry;
    ///
    /// let registry = ProviderRegistry::new();
    /// assert!(registry.list_providers().is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// 返回全局静态注册表（包含内置供应商）。
    ///
    /// 该注册表由 `src/providers/mod.rs` 在程序启动时完成初始化。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::registry::provider_registry::ProviderRegistry;
    ///
    /// let registry = ProviderRegistry::global();
    /// let _names = registry.list_providers();
    /// ```
    pub fn global() -> &'static ProviderRegistry {
        &GLOBAL_REGISTRY
    }

    /// 注册一个供应商实例。
    ///
    /// 若同名（忽略大小写）供应商已存在，新的实例会覆盖旧值。
    ///
    /// # 参数
    /// - `provider`: 待注册的供应商对象。
    ///
    /// # 关键逻辑
    /// 统一使用小写键，以保证 `"Alipay"` 与 `"alipay"` 指向同一条记录。
    ///
    /// # 示例
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use beancount_importer_rust::{
    ///     error::ImporterResult,
    ///     interface::provider::Provider,
    ///     model::{
    ///         config::provider::ProviderConfig,
    ///         data::raw_record::RawRecord,
    ///         registry::provider_registry::ProviderRegistry,
    ///         rule::rule_engine::RuleEngine,
    ///         transaction::Transaction,
    ///     },
    /// };
    ///
    /// struct DemoProvider;
    ///
    /// impl Provider for DemoProvider {
    ///     fn name(&self) -> &'static str {
    ///         "demo"
    ///     }
    ///
    ///     fn transform(
    ///         &self,
    ///         _record: RawRecord,
    ///         _rule_engine: &RuleEngine,
    ///         _config: &ProviderConfig,
    ///     ) -> ImporterResult<Option<Transaction>> {
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// let mut registry = ProviderRegistry::new();
    /// registry.register(Arc::new(DemoProvider));
    ///
    /// assert!(registry.get("DEMO").is_some());
    /// ```
    pub fn register(&mut self, provider: Arc<dyn Provider>) {
        self.providers
            .insert(provider.name().to_lowercase(), provider);
    }

    /// 按供应商名称查找实例。
    ///
    /// 查找过程大小写不敏感。
    ///
    /// # 参数
    /// - `name`: 供应商名称。
    ///
    /// # 返回值
    /// - `Some(Arc<dyn Provider>)`: 找到供应商时返回其共享引用。
    /// - `None`: 未找到匹配项。
    ///
    /// # 示例
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use beancount_importer_rust::{
    ///     error::ImporterResult,
    ///     interface::provider::Provider,
    ///     model::{
    ///         config::provider::ProviderConfig,
    ///         data::raw_record::RawRecord,
    ///         registry::provider_registry::ProviderRegistry,
    ///         rule::rule_engine::RuleEngine,
    ///         transaction::Transaction,
    ///     },
    /// };
    ///
    /// struct DemoProvider;
    ///
    /// impl Provider for DemoProvider {
    ///     fn name(&self) -> &'static str {
    ///         "demo"
    ///     }
    ///
    ///     fn transform(
    ///         &self,
    ///         _record: RawRecord,
    ///         _rule_engine: &RuleEngine,
    ///         _config: &ProviderConfig,
    ///     ) -> ImporterResult<Option<Transaction>> {
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// let mut registry = ProviderRegistry::new();
    /// registry.register(Arc::new(DemoProvider));
    ///
    /// assert!(registry.get("demo").is_some());
    /// assert!(registry.get("DEMO").is_some());
    /// assert!(registry.get("unknown").is_none());
    /// ```
    pub fn get(&self, name: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(&name.to_lowercase()).cloned()
    }

    /// 以稳定排序返回所有已注册供应商名称。
    ///
    /// # 返回值
    /// 按字典序升序排列的供应商名称列表。
    ///
    /// # 关键逻辑
    /// 这里显式排序，是为了让 CLI 输出和测试断言具备确定性，
    /// 避免 `HashMap` 的迭代顺序带来不稳定行为。
    ///
    /// # 示例
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// use beancount_importer_rust::{
    ///     error::ImporterResult,
    ///     interface::provider::Provider,
    ///     model::{
    ///         config::provider::ProviderConfig,
    ///         data::raw_record::RawRecord,
    ///         registry::provider_registry::ProviderRegistry,
    ///         rule::rule_engine::RuleEngine,
    ///         transaction::Transaction,
    ///     },
    /// };
    ///
    /// struct BProvider;
    /// struct AProvider;
    ///
    /// impl Provider for BProvider {
    ///     fn name(&self) -> &'static str {
    ///         "B"
    ///     }
    ///
    ///     fn transform(
    ///         &self,
    ///         _record: RawRecord,
    ///         _rule_engine: &RuleEngine,
    ///         _config: &ProviderConfig,
    ///     ) -> ImporterResult<Option<Transaction>> {
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// impl Provider for AProvider {
    ///     fn name(&self) -> &'static str {
    ///         "a"
    ///     }
    ///
    ///     fn transform(
    ///         &self,
    ///         _record: RawRecord,
    ///         _rule_engine: &RuleEngine,
    ///         _config: &ProviderConfig,
    ///     ) -> ImporterResult<Option<Transaction>> {
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// let mut registry = ProviderRegistry::new();
    /// registry.register(Arc::new(BProvider));
    /// registry.register(Arc::new(AProvider));
    ///
    /// assert_eq!(registry.list_providers(), vec!["a", "b"]);
    /// ```
    pub fn list_providers(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.providers.keys().map(|value| value.as_str()).collect();
        names.sort_unstable();
        names
    }
}

/// 默认实现：创建空注册表。
impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
