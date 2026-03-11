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
