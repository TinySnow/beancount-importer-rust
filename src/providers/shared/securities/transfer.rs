use std::collections::HashMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::{
    error::{ImporterError, ImporterResult},
    model::{
        account::{amount::Amount, posting::Posting},
        config::{meta_value::MetaValue, provider::ProviderConfig},
        rule::match_result::MatchResult,
        transaction::Transaction,
    },
    providers::shared::{append_extra_metadata, append_order_id, apply_match_result},
};

use super::{
    DEFAULT_TRANSFER_ASSET_ACCOUNT, SecurityTransformOptions,
    logic::{derive_cash_account, infer_transfer_in},
};

/// Builds a cash transfer transaction for broker <-> bank movements.
/// Account direction can be overridden by rule action (`debit_account`/`credit_account`).
pub(super) fn build_cash_transfer_transaction(
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
    fee: Option<Decimal>,
    tax: Option<Decimal>,
    extra: HashMap<String, String>,
) -> ImporterResult<Transaction> {
    let transfer_amount = amount.ok_or_else(|| {
        ImporterError::Conversion("Missing transfer amount for cash transfer".to_string())
    })?;
    let transfer_in = infer_transfer_in(transaction_type.as_deref(), transfer_amount);
    // Resolve broker cash account from explicit config or derive from asset account.
    let broker_cash_account = config
        .default_cash_account
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| derive_cash_account(config.default_asset_account.as_deref()));
    // Use rule-provided debit/credit accounts first, then provider defaults.
    let (debit_account, credit_account) = if transfer_in {
        (
            match_result
                .debit_account
                .clone()
                .unwrap_or_else(|| broker_cash_account.clone()),
            match_result
                .credit_account
                .clone()
                .unwrap_or_else(|| DEFAULT_TRANSFER_ASSET_ACCOUNT.to_string()),
        )
    } else {
        (
            match_result
                .debit_account
                .clone()
                .unwrap_or_else(|| DEFAULT_TRANSFER_ASSET_ACCOUNT.to_string()),
            match_result
                .credit_account
                .clone()
                .unwrap_or_else(|| broker_cash_account.clone()),
        )
    };

    let fallback_narration = transaction_type
        .clone()
        .unwrap_or_else(|| "Broker transfer".to_string());
    let mut tx = Transaction::new(
        date,
        match_result
            .narration
            .clone()
            .or(narration)
            .unwrap_or(fallback_narration),
    );

    // Keep posting signs canonical: debit positive, credit negative.
    tx = tx.with_posting(Posting::new(debit_account).with_amount(Amount::new(
        transfer_amount.abs(),
        cash_currency.to_string(),
    )));
    tx = tx.with_posting(Posting::new(credit_account).with_amount(Amount::new(
        -transfer_amount.abs(),
        cash_currency.to_string(),
    )));
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
        payee.or(transaction_type),
        config.name.as_deref(),
    );

    Ok(tx)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    use super::build_cash_transfer_transaction;
    use crate::{
        model::{config::provider::ProviderConfig, rule::match_result::MatchResult},
        providers::shared::SecurityTransformOptions,
    };

    #[test]
    fn uses_explicit_default_cash_account_for_cash_transfer() {
        let options = SecurityTransformOptions {
            provider_name: "yinhe",
            default_payee: "Galaxy",
        };
        let mut config = ProviderConfig::default();
        config.default_asset_account = Some("Assets:Broker:Galaxy:Securities".to_string());
        config.default_cash_account = Some("Assets:Invest:Broker:Galaxy:Cash".to_string());

        let tx = build_cash_transfer_transaction(
            options,
            &MatchResult::default(),
            &config,
            NaiveDate::from_ymd_opt(2026, 3, 1).expect("valid date"),
            Some(dec!(1000)),
            "CNY",
            None,
            None,
            Some("in".to_string()),
            None,
            None,
            None,
            HashMap::new(),
        )
        .expect("cash transfer should build");

        let has_custom_cash_account = tx
            .postings
            .iter()
            .any(|posting| posting.account == "Assets:Invest:Broker:Galaxy:Cash");
        assert!(has_custom_cash_account);
    }
}
