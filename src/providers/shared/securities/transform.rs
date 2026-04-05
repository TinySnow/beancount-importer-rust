//! 证券流水到交易对象的统一转换入口。
//!
//! 该模块负责路由与编排，不直接关心分录细节。
//! 它会在“银证转账”和“证券交易”两条子流程之间做分派。

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
///
/// 该函数返回 `Option<Transaction>`：
/// - `None` 表示被规则忽略；
/// - `Some` 表示成功生成交易。
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

    // 先做语义分类，再路由到对应构建器。
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
