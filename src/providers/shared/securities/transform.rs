//! 模块说明：跨 Provider 的证券交易分类、账户规划与分录构建能力。
//!
//! 文件路径：src/providers/shared/securities/transform.rs。
//! 该文件聚焦原始记录到交易的转换编排。
//! 关键符号：无显式公开符号，主要通过内部实现或模块组织发挥作用。

use crate::{
    error::ImporterResult,
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        rule::rule_engine::RuleEngine, transaction::Transaction,
    },
};

use super::{
    SecurityTransformOptions,
    context::SecurityRecordContext,
    logic::{TransactionKind, classify_transaction_kind},
    normalize::normalize_cash_currency,
    trade::build_security_trade_transaction,
    transfer::build_cash_transfer_transaction,
};

/// 证券类供应商通用转换入口。
///
/// 职责仅包含：
/// 1. 规则匹配与忽略判断；
/// 2. 构建标准证券上下文；
/// 3. 在“银证转账”和“证券交易”之间路由。
pub(crate) fn transform_security_record(
    options: SecurityTransformOptions,
    record: RawRecord,
    rule_engine: &RuleEngine,
    config: &ProviderConfig,
) -> ImporterResult<Option<Transaction>> {
    let match_result = rule_engine.match_record(&record);
    if match_result.ignore {
        return Ok(None);
    }

    let cash_currency = normalize_cash_currency(
        record
            .currency
            .as_deref()
            .or(config.default_currency.as_deref())
            .unwrap_or("CNY"),
    );

    let context = SecurityRecordContext::from_record(record, cash_currency)?;

    if classify_transaction_kind(
        context.transaction_type.as_deref(),
        context.symbol.as_deref(),
    ) == TransactionKind::CashTransfer
    {
        let tx = build_cash_transfer_transaction(options, &match_result, config, context)?;
        return Ok(Some(tx));
    }

    let tx = build_security_trade_transaction(options, &match_result, config, context)?;
    Ok(Some(tx))
}
