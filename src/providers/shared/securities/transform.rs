use crate::{
    error::{ImporterError, ImporterResult},
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        rule::rule_engine::RuleEngine, transaction::Transaction,
    },
};

use super::{
    SecurityTransformOptions, logic::is_cash_transfer_record, normalize::normalize_cash_currency,
    trade::build_security_trade_transaction, transfer::build_cash_transfer_transaction,
};

/// 证券类供应商通用转换入口。
///
/// 该入口仅负责：
/// - 规则匹配与忽略判断。
/// - 通用字段解包与基础校验。
/// - 在“银证转账”与“证券交易”之间路由。
///
/// 具体分录构建逻辑已下沉到子模块，便于后续扩展股票、期权等场景。
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

    let RawRecord {
        date,
        amount,
        currency,
        payee,
        narration,
        transaction_type,
        reference,
        symbol,
        security_name,
        quantity,
        unit_price,
        fee,
        tax,
        extra,
        ..
    } = record;

    let date = date.ok_or_else(|| ImporterError::Conversion("Missing trade date".to_string()))?;

    let cash_currency = normalize_cash_currency(
        currency
            .as_deref()
            .or(config.default_currency.as_deref())
            .unwrap_or("CNY"),
    );

    if is_cash_transfer_record(transaction_type.as_deref(), symbol.as_deref()) {
        let tx = build_cash_transfer_transaction(
            options,
            &match_result,
            config,
            date,
            amount,
            &cash_currency,
            payee,
            narration,
            transaction_type,
            reference,
            fee,
            tax,
            extra,
        )?;
        return Ok(Some(tx));
    }

    let tx = build_security_trade_transaction(
        options,
        &match_result,
        config,
        date,
        amount,
        &cash_currency,
        payee,
        narration,
        transaction_type,
        reference,
        symbol,
        security_name,
        quantity,
        unit_price,
        fee,
        tax,
        extra,
    )?;

    Ok(Some(tx))
}
