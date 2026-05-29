//! 京东账单导入适配器。
//!
//! 该模块实现第三方支付供应商 `jd`，并复用共享现金流转换器完成：
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
//!     providers::third_party::jd::JdProvider,
//! };
//!
//! let provider = JdProvider;
//! assert_eq!(provider.name(), "jd");
//! assert_eq!(provider.description(), "JD statement importer");
//!
//! let config = ProviderConfig::default();
//! let global = GlobalConfig::default();
//! let provider_rules: [Rule; 0] = [];
//! let rule_engine = RuleEngine::new(&provider_rules, &global);
//! let record = RawRecord::new();
//! let _ = provider.transform(record, &rule_engine, &config)?;
//! # Ok::<(), beancount_importer_rust::error::ImporterError>(())
//! ```

use std::path::Path;

use crate::{
    error::ImporterResult,
    interface::provider::{Provider, parse_tabular_source},
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        mapping::field_mapping::FieldMapping, rule::rule_engine::RuleEngine,
        transaction::Transaction,
    },
    providers::shared::{CashflowTransformOptions, transform_cashflow_record},
};

/// 京东现金流转换参数。
///
/// 该常量用于向共享转换器注入默认资产账户兜底值。
const JD_OPTIONS: CashflowTransformOptions = CashflowTransformOptions {
    default_asset_fallback: "Assets:JD",
};

/// 京东账单 `Provider`。
///
/// 该类型为零大小类型（ZST），不保留运行时状态。
pub struct JdProvider;

impl Provider for JdProvider {
    /// 返回供应商唯一标识：`"jd"`。
    fn name(&self) -> &'static str {
        "jd"
    }

    /// 返回供应商说明文本。
    fn description(&self) -> &'static str {
        "JD statement importer"
    }

    fn display_name(&self) -> &'static str {
        "京东"
    }

    fn parse(
        &self,
        path: &Path,
        mapping: &FieldMapping,
        config: &ProviderConfig,
        strict_mode: bool,
    ) -> ImporterResult<Vec<RawRecord>> {
        parse_tabular_source(path, mapping, config, strict_mode)
    }

    /// 将一条京东原始记录转换为交易。
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
        transform_cashflow_record(
            self.name(),
            self.display_name(),
            JD_OPTIONS,
            record,
            rule_engine,
            config,
        )
    }
}
