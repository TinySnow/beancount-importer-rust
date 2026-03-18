//! 模块说明：第三方支付平台 Provider 适配实现。
//!
//! 文件路径：src/providers/third_party/mt.rs。
//! 该文件围绕 'mt' 的职责提供实现。
//! 关键符号：MT_OPTIONS、MtProvider、name、description。

use crate::{
    error::ImporterResult,
    interface::provider::Provider,
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        rule::rule_engine::RuleEngine, transaction::Transaction,
    },
    providers::shared::{CashflowTransformOptions, transform_cashflow_record},
};

const MT_OPTIONS: CashflowTransformOptions = CashflowTransformOptions {
    provider_name: "mt",
    default_asset_fallback: "Assets:Meituan",
};

pub struct MtProvider;

impl Provider for MtProvider {
    fn name(&self) -> &'static str {
        "mt"
    }

    fn description(&self) -> &'static str {
        "Meituan statement importer"
    }

    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        transform_cashflow_record(MT_OPTIONS, record, rule_engine, config)
    }
}
