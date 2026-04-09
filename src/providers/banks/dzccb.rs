//! 达州银行（DZCCB）账单导入适配器。
//!
//! 本模块通过 `Provider` trait 暴露达州银行导入能力，并将具体转换流程
//! 委托给共享现金流转换逻辑，以统一银行类账单的处理语义。
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
//!     providers::banks::dzccb::DzccbProvider,
//! };
//!
//! let provider = DzccbProvider;
//! assert_eq!(provider.name(), "dzccb");
//! assert_eq!(
//!     provider.description(),
//!     "DaZhou City Commercial Bank statement importer"
//! );
//!
//! let config = ProviderConfig::default();
//! let global = GlobalConfig::default();
//! let provider_rules: [Rule; 0] = [];
//! let rule_engine = RuleEngine::new(&provider_rules, &global);
//! let record = RawRecord::new();
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
    providers::shared::{CashflowTransformOptions, transform_cashflow_record},
};

/// DZCCB 现金流转换参数。
///
/// 该常量将资产账户兜底值绑定到共享转换入口。
const DZCCB_OPTIONS: CashflowTransformOptions = CashflowTransformOptions {
    default_asset_fallback: "Assets:DZCCB",
};

/// 达州银行账单 `Provider`。
///
/// 作为 `unit struct`，其职责是提供银行标识与参数选择，不保存状态。
pub struct DzccbProvider;

impl Provider for DzccbProvider {
    /// 返回供应商唯一标识：`"dzccb"`。
    fn name(&self) -> &'static str {
        "dzccb"
    }

    /// 返回供应商描述信息。
    fn description(&self) -> &'static str {
        "DaZhou City Commercial Bank statement importer"
    }

    /// 将原始记录转换为标准交易。
    ///
    /// 关键逻辑：通过共享转换函数统一处理收支分类、账户选择和元数据注入，
    /// 本地仅指定 DZCCB 的参数差异。
    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        // 委托共享转换逻辑，避免不同银行重复实现现金流转换流程。
        transform_cashflow_record(self.name(), DZCCB_OPTIONS, record, rule_engine, config)
    }
}
