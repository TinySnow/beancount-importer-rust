//! 支付宝 Provider。

use crate::{
    error::ImporterResult,
    interface::provider::Provider,
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        rule::rule_engine::RuleEngine, transaction::Transaction,
    },
    providers::shared::{CashflowTransformOptions, transform_cashflow_record},
};

const ALIPAY_OPTIONS: CashflowTransformOptions = CashflowTransformOptions {
    provider_name: "alipay",
    default_asset_fallback: "Assets:Alipay",
};

pub struct AlipayProvider;

impl Provider for AlipayProvider {
    fn name(&self) -> &'static str {
        "alipay"
    }

    fn description(&self) -> &'static str {
        "Alipay statement importer"
    }

    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        transform_cashflow_record(ALIPAY_OPTIONS, record, rule_engine, config)
    }
}
