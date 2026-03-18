//! 模块说明：证券对账单 Provider 适配实现。
//!
//! 文件路径：src/providers/securities/futu.rs。
//! 该文件围绕 'futu' 的职责提供实现。
//! 关键符号：FUTU_OPTIONS、FutuProvider、name、description。

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
