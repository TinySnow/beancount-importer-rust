use crate::{
    error::ImporterResult,
    interface::provider::Provider,
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        rule::rule_engine::RuleEngine, transaction::Transaction,
    },
    providers::shared::{CashflowTransformOptions, transform_cashflow_record},
};

const ICBC_OPTIONS: CashflowTransformOptions = CashflowTransformOptions {
    provider_name: "icbc",
    default_asset_fallback: "Assets:ICBC",
};

pub struct IcbcProvider;

impl Provider for IcbcProvider {
    fn name(&self) -> &'static str {
        "icbc"
    }

    fn description(&self) -> &'static str {
        "ICBC statement importer"
    }

    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        transform_cashflow_record(ICBC_OPTIONS, record, rule_engine, config)
    }
}
