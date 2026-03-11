use crate::{
    error::ImporterResult,
    interface::provider::Provider,
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        rule::rule_engine::RuleEngine, transaction::Transaction,
    },
    providers::shared::{CashflowTransformOptions, transform_cashflow_record},
};

const JD_OPTIONS: CashflowTransformOptions = CashflowTransformOptions {
    provider_name: "jd",
    default_asset_fallback: "Assets:JD",
};

pub struct JdProvider;

impl Provider for JdProvider {
    fn name(&self) -> &'static str {
        "jd"
    }

    fn description(&self) -> &'static str {
        "JD statement importer"
    }

    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        transform_cashflow_record(JD_OPTIONS, record, rule_engine, config)
    }
}
