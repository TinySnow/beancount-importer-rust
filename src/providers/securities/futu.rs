//! 富途证券 Provider。

use crate::{
    error::ImporterResult,
    interface::provider::Provider,
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        rule::rule_engine::RuleEngine, transaction::Transaction,
    },
    providers::shared::{SecurityTransformOptions, transform_security_record},
};

const FUTU_OPTIONS: SecurityTransformOptions = SecurityTransformOptions {
    provider_name: "futu",
    default_payee: "Futu",
};

pub struct FutuProvider;

impl Provider for FutuProvider {
    fn name(&self) -> &'static str {
        "futu"
    }

    fn description(&self) -> &'static str {
        "Futu securities statement importer"
    }

    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        transform_security_record(FUTU_OPTIONS, record, rule_engine, config)
    }
}
