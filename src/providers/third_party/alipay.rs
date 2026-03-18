//! 模块说明：第三方支付平台 Provider 适配实现。
//!
//! 文件路径：src/providers/third_party/alipay.rs。
//! 该文件围绕 'alipay' 的职责提供实现。
//! 关键符号：ALIPAY_OPTIONS、AlipayProvider、name、description。

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
