use crate::{
    error::ImporterResult,
    interface::provider::Provider,
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        rule::rule_engine::RuleEngine, transaction::Transaction,
    },
    providers::shared::{CashflowTransformOptions, transform_cashflow_record},
};

const WECHAT_OPTIONS: CashflowTransformOptions = CashflowTransformOptions {
    provider_name: "wechat",
    default_asset_fallback: "Assets:WeChat",
};

pub struct WechatProvider;

impl Provider for WechatProvider {
    fn name(&self) -> &'static str {
        "wechat"
    }

    fn description(&self) -> &'static str {
        "WeChat statement importer"
    }

    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        transform_cashflow_record(WECHAT_OPTIONS, record, rule_engine, config)
    }
}
