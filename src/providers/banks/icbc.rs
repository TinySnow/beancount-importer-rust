//! 中国工商银行（ICBC）账单导入适配器。
//!
//! 该模块实现银行账单供应商 `icbc`，并复用共享现金流转换器完成：
//! - 规则匹配与忽略判定；
//! - 收支方向推断与双分录生成；
//! - 订单号与扩展字段元数据附加。
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
//!     providers::banks::icbc::IcbcProvider,
//! };
//!
//! let provider = IcbcProvider;
//! assert_eq!(provider.name(), "icbc");
//! assert_eq!(provider.description(), "ICBC statement importer");
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

/// ICBC 现金流转换参数。
///
/// 该常量定义了共享转换流程所需的银行特定配置。
const ICBC_OPTIONS: CashflowTransformOptions = CashflowTransformOptions {
    default_asset_fallback: "Assets:ICBC",
};

/// ICBC 账单 `Provider`。
///
/// 使用零大小类型（ZST）实现，无需存储内部状态。
pub struct IcbcProvider;

impl Provider for IcbcProvider {
    /// 返回供应商唯一标识：`"icbc"`。
    fn name(&self) -> &'static str {
        "icbc"
    }

    /// 返回供应商说明文本。
    fn description(&self) -> &'static str {
        "ICBC statement importer"
    }

    /// 将一条原始账单记录转换为交易。
    ///
    /// 关键逻辑：适配器仅负责注入 ICBC 配置，具体转换步骤由共享现金流模块统一执行。
    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        // 复用共享转换实现，减少银行 Provider 之间的逻辑分叉。
        transform_cashflow_record(self.name(), ICBC_OPTIONS, record, rule_engine, config)
    }
}
