use crate::{
    error::{ImporterError, ImporterResult},
    model::{
        config::{meta_value::MetaValue, provider::ProviderConfig},
        rule::match_result::MatchResult,
        transaction::Transaction,
    },
    providers::shared::{append_extra_metadata, append_order_id, apply_match_result},
};

use super::{
    SecurityTransformOptions,
    context::SecurityRecordContext,
    logic::{TradeDirection, infer_trade_direction, is_repo_trade},
    normalize::normalize_security_commodity,
    trade_accounts::build_trade_account_plan,
    trade_repo::{RepoPostingInput, apply_repo_postings},
    trade_spot::{SpotPostingInput, apply_spot_postings},
};

/// 构建证券交易分录。
///
/// 输入使用 `SecurityRecordContext` 承载，避免长参数链路。
pub(super) fn build_security_trade_transaction(
    options: SecurityTransformOptions,
    match_result: &MatchResult,
    config: &ProviderConfig,
    context: SecurityRecordContext,
) -> ImporterResult<Transaction> {
    let SecurityRecordContext {
        date,
        amount,
        cash_currency,
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
    } = context;

    let symbol = symbol
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ImporterError::Conversion("Missing security symbol".to_string()))?;

    let quantity =
        quantity.ok_or_else(|| ImporterError::Conversion("Missing quantity".to_string()))?;

    let commodity_symbol = normalize_security_commodity(
        &symbol,
        transaction_type.as_deref(),
        security_name.as_deref(),
    );

    let narration = match_result
        .narration
        .clone()
        .or(narration)
        .unwrap_or_else(|| format!("Trade {}", symbol));

    let tx = Transaction::new(date, narration);

    let trade_direction = infer_trade_direction(transaction_type.as_deref(), amount);
    let is_buy = trade_direction == TradeDirection::Buy;
    let repo_trade = is_repo_trade(&symbol, transaction_type.as_deref());
    let account_plan = build_trade_account_plan(match_result, config, is_buy);

    let cash_amount = match amount {
        Some(value) => value.abs(),
        None => {
            let price = unit_price.ok_or_else(|| {
                ImporterError::Conversion(
                    "Missing cash amount and unit price for securities trade".to_string(),
                )
            })?;
            quantity.abs() * price
        }
    };

    let signed_quantity = if is_buy {
        quantity.abs()
    } else {
        -quantity.abs()
    };
    let signed_cash = if is_buy { -cash_amount } else { cash_amount };

    let mut tx = if repo_trade {
        apply_repo_postings(RepoPostingInput {
            tx,
            holdings_account: &account_plan.holdings_account,
            cash_account: &account_plan.cash_account,
            commodity_symbol: &commodity_symbol,
            cash_currency: &cash_currency,
            signed_quantity,
            signed_cash,
            quantity,
            cash_amount,
            is_buy,
            fee_account: &account_plan.fee_account,
            rounding_account: &account_plan.rounding_account,
            interest_account: &account_plan.interest_account,
        })
    } else {
        let effective_price = match unit_price {
            Some(price) => price,
            None => {
                if quantity.is_zero() {
                    return Err(ImporterError::Conversion(
                        "Missing unit price and quantity is zero".to_string(),
                    ));
                }
                cash_amount / quantity.abs()
            }
        };

        apply_spot_postings(SpotPostingInput {
            tx,
            holdings_account: &account_plan.holdings_account,
            cash_account: &account_plan.cash_account,
            commodity_symbol: &commodity_symbol,
            cash_currency: &cash_currency,
            signed_quantity,
            signed_cash,
            quantity,
            cash_amount,
            is_buy,
            effective_price,
            fee_account: &account_plan.fee_account,
            rounding_account: &account_plan.rounding_account,
            pnl_account: &account_plan.pnl_account,
        })
    };

    tx = tx.with_meta("symbol", MetaValue::String(symbol));
    if let Some(security_name) = security_name {
        tx = tx.with_meta("securityName", MetaValue::String(security_name));
    }
    if let Some(fee) = fee {
        tx = tx.with_meta("fee", MetaValue::Number(fee));
    }
    if let Some(tax) = tax {
        tx = tx.with_meta("tax", MetaValue::Number(tax));
    }

    tx = append_order_id(tx, options.provider_name, reference);
    tx = append_extra_metadata(tx, options.provider_name, extra);
    tx = apply_match_result(
        tx,
        options.provider_name,
        match_result,
        payee.or_else(|| Some(options.default_payee.to_string())),
        config.name.as_deref(),
    );

    Ok(tx)
}

#[cfg(test)]
mod tests;
