//! 富途证券（Futu）账单导入适配器。
//!
//! 该模块实现证券供应商 `futu`，并复用共享证券转换器完成：
//! - 规则匹配与忽略判定；
//! - 银证转账与证券交易类型路由；
//! - 过账构建、订单号与扩展字段元数据附加。
//!
//! # 示例
//! ```rust,no_run
//! use beancount_importer_rust::{
//!     interface::provider::Provider,
//!     model::{
//!         config::{global::GlobalConfig, provider::ProviderConfig},
//!         data::raw_record::RawRecord,
//!         rule::{Rule, rule_engine::RuleEngine},
//!     },
//!     providers::securities::futu::FutuProvider,
//! };
//!
//! let provider = FutuProvider;
//! assert_eq!(provider.name(), "futu");
//! assert_eq!(provider.description(), "Futu securities statement importer");
//!
//! let config = ProviderConfig::default();
//! let global = GlobalConfig::default();
//! let provider_rules: [Rule; 0] = [];
//! let rule_engine = RuleEngine::new(&provider_rules, &global);
//! let record = RawRecord::new();
//!
//! let _ = provider.transform(record, &rule_engine, &config)?;
//! # Ok::<(), beancount_importer_rust::error::ImporterError>(())
//! ```

use crate::{
    error::ImporterResult,
    interface::provider::Provider,
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        rule::rule_engine::RuleEngine, transaction::Transaction,
    },
    providers::shared::{SecurityTransformOptions, transform_security_record},
};

/// 富途证券共享转换参数。
const FUTU_OPTIONS: SecurityTransformOptions = SecurityTransformOptions {
    default_payee: "Futu",
};

/// 富途证券账单 `Provider`。
///
/// 该实现是无状态零大小类型（ZST），仅负责注入富途专属参数。
pub struct FutuProvider;

impl Provider for FutuProvider {
    /// 返回供应商唯一标识：`"futu"`。
    fn name(&self) -> &'static str {
        "futu"
    }

    /// 返回供应商描述信息。
    fn description(&self) -> &'static str {
        "Futu securities statement importer"
    }

    fn display_name(&self) -> &'static str {
        "富途"
    }

    /// 将一条富途原始记录转换为交易。
    ///
    /// 关键逻辑：富途侧不额外分叉业务判断，统一走共享证券转换流水线。
    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        // 将富途标识与默认 payee 注入共享转换层，避免 Provider 间重复实现。
        transform_security_record(self.name(), self.display_name(), FUTU_OPTIONS, record, rule_engine, config)
    }
}
