//! 美团账单导入适配器。
//!
//! 该模块实现第三方支付供应商 `mt`，并复用共享现金流转换器完成：
//! - 规则匹配与忽略判定；
//! - 收支方向推断与双分录构建；
//! - 订单号和扩展字段元数据附加。
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
//!     providers::third_party::mt::MtProvider,
//! };
//!
//! let provider = MtProvider;
//! assert_eq!(provider.name(), "mt");
//! assert_eq!(provider.description(), "Meituan statement importer");
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

/// 美团现金流转换参数。
///
/// 该常量用于向共享转换器注入默认资产账户兜底值。
const MT_OPTIONS: CashflowTransformOptions = CashflowTransformOptions {
    default_asset_fallback: "Assets:Meituan",
};

/// 美团账单 `Provider`。
///
/// 该类型为零大小类型（ZST），不保留运行时状态。
pub struct MtProvider;

impl Provider for MtProvider {
    /// 返回供应商唯一标识：`"mt"`。
    fn name(&self) -> &'static str {
        "mt"
    }

    /// 返回供应商说明文本。
    fn description(&self) -> &'static str {
        "Meituan statement importer"
    }

    /// 将一条美团原始记录转换为交易。
    ///
    /// 关键逻辑：适配器仅提供平台参数，
    /// 具体转换流程由共享现金流转换器统一处理。
    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        // 复用共享转换实现，保持第三方支付 Provider 行为一致。
        transform_cashflow_record(self.name(), MT_OPTIONS, record, rule_engine, config)
    }
}
