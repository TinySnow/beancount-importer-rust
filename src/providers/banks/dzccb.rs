//! 模块说明：银行对账单 Provider 适配实现。
//!
//! 文件路径：src/providers/banks/dzccb.rs。
//! 该文件围绕 'dzccb' 的职责提供实现。
//! 关键符号：DZCCB_OPTIONS、DzccbProvider、name、description。

use crate::{
    error::ImporterResult,
    interface::provider::Provider,
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        rule::rule_engine::RuleEngine, transaction::Transaction,
    },
    providers::shared::{CashflowTransformOptions, transform_cashflow_record},
};

const DZCCB_OPTIONS: CashflowTransformOptions = CashflowTransformOptions {
    provider_name: "dzccb",
    default_asset_fallback: "Assets:DZCCB",
};

pub struct DzccbProvider;

impl Provider for DzccbProvider {
    fn name(&self) -> &'static str {
        "dzccb"
    }

    fn description(&self) -> &'static str {
        "DaZhou City Commercial Bank statement importer"
    }

    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        transform_cashflow_record(DZCCB_OPTIONS, record, rule_engine, config)
    }
}
