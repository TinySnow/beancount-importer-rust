//! 银河证券 Provider。

use crate::{
    error::ImporterResult,
    interface::provider::Provider,
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        rule::rule_engine::RuleEngine, transaction::Transaction,
    },
    providers::shared::{SecurityTransformOptions, transform_security_record},
};

const YINHE_OPTIONS: SecurityTransformOptions = SecurityTransformOptions {
    provider_name: "yinhe",
    default_payee: "Galaxy",
};

pub struct YinheProvider;

impl Provider for YinheProvider {
    fn name(&self) -> &'static str {
        "yinhe"
    }

    fn description(&self) -> &'static str {
        "Yinhe securities statement importer"
    }

    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        transform_security_record(YINHE_OPTIONS, record, rule_engine, config)
    }
}
