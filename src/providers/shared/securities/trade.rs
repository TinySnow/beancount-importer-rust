use std::collections::HashMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;

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
    logic::{derive_cash_account, derive_rounding_account, infer_is_buy, is_repo_trade},
    normalize::normalize_security_commodity,
    trade_repo::{RepoPostingInput, apply_repo_postings},
    trade_spot::{SpotPostingInput, apply_spot_postings},
};

/// 构建证券买卖/逆回购交易。
pub(super) fn build_security_trade_transaction(
    options: SecurityTransformOptions,
    match_result: &MatchResult,
    config: &ProviderConfig,
    date: NaiveDate,
    amount: Option<Decimal>,
    cash_currency: &str,
    payee: Option<String>,
    narration: Option<String>,
    transaction_type: Option<String>,
    reference: Option<String>,
    symbol: Option<String>,
    security_name: Option<String>,
    quantity: Option<Decimal>,
    unit_price: Option<Decimal>,
    fee: Option<Decimal>,
    tax: Option<Decimal>,
    extra: HashMap<String, String>,
) -> ImporterResult<Transaction> {
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

    let is_buy = infer_is_buy(transaction_type.as_deref(), amount);
    let is_repo_trade = is_repo_trade(&symbol, transaction_type.as_deref());

    let holdings_account = if is_buy {
        match_result
            .debit_account
            .clone()
            .or(config.default_asset_account.clone())
            .unwrap_or_else(|| "Assets:Investments".to_string())
    } else {
        match_result
            .credit_account
            .clone()
            .or(config.default_asset_account.clone())
            .unwrap_or_else(|| "Assets:Investments".to_string())
    };

    // 兜底现金账户从 default_asset_account 推导，避免落到通用 Assets:Cash。
    let broker_cash_account = derive_cash_account(config.default_asset_account.as_deref());

    let cash_account = if is_buy {
        match_result
            .credit_account
            .clone()
            .unwrap_or_else(|| broker_cash_account.clone())
    } else {
        match_result
            .debit_account
            .clone()
            .unwrap_or_else(|| broker_cash_account.clone())
    };

    let fee_account = match_result
        .fee_account
        .clone()
        .or(config.default_fee_account.clone())
        .or(config.default_expense_account.clone())
        .unwrap_or_else(|| "Expenses:Investing:Fees".to_string());
    let rounding_account = match_result
        .rounding_account
        .clone()
        .or(config.default_rounding_account.clone())
        .unwrap_or_else(|| derive_rounding_account(&fee_account));
    let pnl_account = match_result
        .pnl_account
        .clone()
        .or(config.default_pnl_account.clone())
        .or(config.default_income_account.clone())
        .filter(|value| value != "Income:Unknown")
        .unwrap_or_else(|| "Income:Investing:Capital-Gains".to_string());
    let interest_account = "Income:Investing:Interest".to_string();

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

    let mut tx = if is_repo_trade {
        apply_repo_postings(RepoPostingInput {
            tx,
            holdings_account: &holdings_account,
            cash_account: &cash_account,
            commodity_symbol: &commodity_symbol,
            cash_currency,
            signed_quantity,
            signed_cash,
            quantity,
            cash_amount,
            is_buy,
            fee_account: &fee_account,
            rounding_account: &rounding_account,
            interest_account: &interest_account,
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
            holdings_account: &holdings_account,
            cash_account: &cash_account,
            commodity_symbol: &commodity_symbol,
            cash_currency,
            signed_quantity,
            signed_cash,
            quantity,
            cash_amount,
            is_buy,
            effective_price,
            fee_account: &fee_account,
            rounding_account: &rounding_account,
            pnl_account: &pnl_account,
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
mod tests {
    use std::collections::HashMap;

    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    use super::build_security_trade_transaction;
    use crate::{
        model::{config::provider::ProviderConfig, rule::match_result::MatchResult},
        providers::shared::SecurityTransformOptions,
    };

    #[test]
    fn uses_broker_cash_account_as_fallback_in_security_trade() {
        let options = SecurityTransformOptions {
            provider_name: "yinhe",
            default_payee: "Galaxy",
        };
        let mut config = ProviderConfig::default();
        config.default_asset_account = Some("Assets:Broker:Galaxy:Securities".to_string());

        let tx = build_security_trade_transaction(
            options,
            &MatchResult::default(),
            &config,
            NaiveDate::from_ymd_opt(2026, 3, 1).expect("valid date"),
            Some(dec!(1000)),
            "CNY",
            None,
            Some("证券买入".to_string()),
            Some("证券买入".to_string()),
            None,
            Some("159915".to_string()),
            Some("某ETF".to_string()),
            Some(dec!(100)),
            Some(dec!(10)),
            None,
            None,
            HashMap::new(),
        )
        .expect("trade should build");

        let has_expected_cash_account = tx
            .postings
            .iter()
            .any(|posting| posting.account == "Assets:Broker:Galaxy:Cash");
        assert!(has_expected_cash_account);
    }
}
