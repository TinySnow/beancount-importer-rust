//! 证券交易分录构建主流程。
//!
//! 该模块负责把证券交易上下文转换为最终 `Transaction`，并根据交易语义
//! 选择现货或逆回购构建路径。

mod accounts;
mod repo;
mod spot;

use crate::{
    error::{ImporterError, ImporterResult},
    model::{
        account::{amount::Amount, cost::Cost, posting::Posting},
        config::{meta_value::MetaValue, provider::ProviderConfig},
        rule::match_result::MatchResult,
        transaction::Transaction,
    },
    providers::shared::{append_extra_metadata, append_order_id, apply_match_result},
};

use self::{
    accounts::build_trade_account_plan,
    repo::{RepoPostingInput, apply_repo_postings},
    spot::{SpotPostingInput, apply_spot_postings},
};
use super::{
    SecurityTransformOptions,
    context::SecurityRecordContext,
    logic::{TradeDirection, infer_trade_direction, is_repo_trade, is_split_trade},
    normalize::normalize_security_commodity,
};

/// 构建证券交易分录。
///
/// 输入使用 `SecurityRecordContext` 承载，避免长参数链路。
/// 函数会在必要字段缺失时返回 `ImporterError::Conversion`。
pub(super) fn build_security_trade_transaction(
    provider_name: &str,
    display_name: &str,
    options: SecurityTransformOptions,
    match_result: &MatchResult,
    config: &ProviderConfig,
    context: SecurityRecordContext,
) -> ImporterResult<Transaction> {
    // ETF 份额分拆：同时移除旧份额和添加新份额，无现金无 PnL
    if is_split_trade(context.transaction_type.as_deref()) {
        return build_split_transaction(
            provider_name, display_name, options, match_result, config, context,
        );
    }

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

    // 交易方向与交易类型共同决定账户选型和金额符号。
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

    // 统一符号语义：买入数量为正/现金为负；卖出相反。
    let signed_quantity = if is_buy {
        quantity.abs()
    } else {
        -quantity.abs()
    };
    let signed_cash = if is_buy { -cash_amount } else { cash_amount };

    // 逆回购与现货交易在持仓成本和差额归因上规则不同，拆分为两套构建器。
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
        // 未给出单价时，允许由现金总额与数量反推单价。
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

    tx = append_order_id(tx, provider_name, reference);
    tx = append_extra_metadata(tx, provider_name, extra);
    tx = apply_match_result(
        tx,
        provider_name,
        match_result,
        payee.or_else(|| Some(options.default_payee.to_string())),
        display_name,
    );

    Ok(tx)
}

/// 构建 ETF 份额分拆交易：移除旧份额 + 添加新份额，无现金无 PnL。
fn build_split_transaction(
    provider_name: &str,
    display_name: &str,
    options: SecurityTransformOptions,
    match_result: &MatchResult,
    config: &ProviderConfig,
    context: SecurityRecordContext,
) -> ImporterResult<Transaction> {
    let date = context.date;
    let cash_currency = context.cash_currency;
    let symbol = context.symbol.as_deref().unwrap_or("").to_string();
    let security_name = context.security_name.clone();
    let quantity = context.quantity.unwrap_or_default();
    let narration = match_result.narration.clone()
        .or(context.narration)
        .unwrap_or_else(|| format!("Split {}", symbol));

    let commodity_symbol = normalize_security_commodity(
        &symbol,
        context.transaction_type.as_deref(),
        security_name.as_deref(),
    );

    let account_plan = build_trade_account_plan(match_result, config, false);

    // 从 netPnl 获取总成本：netPnl 一般为负值，绝对值 = 原始 lot 总成本
    let total_cost = context.extra.get("netPnl")
        .and_then(|v| rust_decimal::Decimal::from_str_exact(v).ok())
        .map(|v| v.abs())
        .unwrap_or_default();

    // 新份额数 = position（分拆后持仓量）
    let new_quantity = context.extra.get("position")
        .and_then(|v| rust_decimal::Decimal::from_str_exact(v).ok())
        .unwrap_or(quantity);

    // 新成本价 = 总成本 / 新份额数
    let new_unit_cost = if !new_quantity.is_zero() && !total_cost.is_zero() {
        (total_cost / new_quantity).round_dp(4)
    } else {
        rust_decimal::Decimal::ZERO
    };

    let mut tx = Transaction::new(date, narration);

    // 移除旧份额（用 {} 让 inventory 系统匹配原始 lot）
    tx = tx.with_posting(
        Posting::new(&account_plan.holdings_account)
            .with_amount(Amount::new(-quantity.abs(), commodity_symbol.clone()))
            .with_inferred_cost(),
    );

    // 添加新份额（调整后成本）
    if !new_unit_cost.is_zero() {
        tx = tx.with_posting(
            Posting::new(&account_plan.holdings_account)
                .with_amount(Amount::new(new_quantity, commodity_symbol.clone()))
                .with_cost(Cost::new(new_unit_cost, cash_currency.clone())),
        );
    } else {
        tx = tx.with_posting(
            Posting::new(&account_plan.holdings_account)
                .with_amount(Amount::new(new_quantity, commodity_symbol)),
        );
    }

    tx = tx.with_meta("symbol", MetaValue::String(symbol));
    if let Some(name) = security_name {
        tx = tx.with_meta("securityName", MetaValue::String(name));
    }

    tx = append_order_id(tx, provider_name, context.reference);
    tx = append_extra_metadata(tx, provider_name, context.extra);
    tx = apply_match_result(
        tx, provider_name, match_result,
        context.payee.or_else(|| Some(options.default_payee.to_string())),
        display_name,
    );

    Ok(tx)
}

#[cfg(test)]
mod tests;
