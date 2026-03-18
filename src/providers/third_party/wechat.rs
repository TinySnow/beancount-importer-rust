//! 模块说明：第三方支付平台 Provider 适配实现。
//!
//! 文件路径：src/providers/third_party/wechat.rs。
//! 该文件围绕 'wechat' 的职责提供实现。
//! 关键符号：WECHAT_OPTIONS、WechatProvider、name、description。

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
