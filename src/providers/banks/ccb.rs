use crate::{
    error::ImporterResult,
    interface::provider::Provider,
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        rule::rule_engine::RuleEngine, transaction::Transaction,
    },
    providers::shared::{CashflowTransformOptions, transform_cashflow_record},
};

const CCB_OPTIONS: CashflowTransformOptions = CashflowTransformOptions {
    provider_name: "ccb",
    default_asset_fallback: "Assets:CCB",
};

pub struct CcbProvider;

impl Provider for CcbProvider {
    fn name(&self) -> &'static str {
        "ccb"
    }

    fn description(&self) -> &'static str {
        "CCB statement importer"
    }

    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        transform_cashflow_record(CCB_OPTIONS, record, rule_engine, config)
    }
}
