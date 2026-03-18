use std::collections::HashMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::super::context::SecurityRecordContext;
use super::build_security_trade_transaction;
use crate::{
    model::{
        config::provider::{ProviderConfig, SecuritiesAccountsConfig},
        rule::match_result::MatchResult,
    },
    providers::shared::SecurityTransformOptions,
};

fn make_context(
    amount: Decimal,
    tx_type: &str,
    symbol: &str,
    quantity: Decimal,
    unit_price: Option<Decimal>,
) -> SecurityRecordContext {
    SecurityRecordContext {
        date: NaiveDate::from_ymd_opt(2026, 3, 1).expect("valid date"),
        amount: Some(amount),
        cash_currency: "CNY".to_string(),
        payee: None,
        narration: Some("security trade".to_string()),
        transaction_type: Some(tx_type.to_string()),
        reference: None,
        symbol: Some(symbol.to_string()),
        security_name: Some("ETF".to_string()),
        quantity: Some(quantity),
        unit_price,
        fee: None,
        tax: None,
        extra: HashMap::new(),
    }
}

#[test]
fn uses_broker_cash_account_as_fallback_in_security_trade() {
    let options = SecurityTransformOptions {
        provider_name: "yinhe",
        default_payee: "Galaxy",
    };
    let config = ProviderConfig {
        default_asset_account: Some("Assets:Broker:Galaxy:Securities".to_string()),
        ..ProviderConfig::default()
    };

    let tx = build_security_trade_transaction(
        options,
        &MatchResult::default(),
        &config,
        make_context(dec!(1000), "buy", "159915", dec!(100), Some(dec!(10))),
    )
    .expect("trade should build");

    let has_expected_cash_account = tx
        .postings
        .iter()
        .any(|posting| posting.account == "Assets:Broker:Galaxy:Cash");
    assert!(has_expected_cash_account);
}

#[test]
fn uses_explicit_default_cash_account_when_configured() {
    let options = SecurityTransformOptions {
        provider_name: "yinhe",
        default_payee: "Galaxy",
    };
    let config = ProviderConfig {
        default_asset_account: Some("Assets:Broker:Galaxy:Securities".to_string()),
        default_cash_account: Some("Assets:Invest:Broker:Galaxy:Cash".to_string()),
        ..ProviderConfig::default()
    };

    let tx = build_security_trade_transaction(
        options,
        &MatchResult::default(),
        &config,
        make_context(dec!(1000), "buy", "159915", dec!(100), Some(dec!(10))),
    )
    .expect("trade should build");

    let has_expected_cash_account = tx
        .postings
        .iter()
        .any(|posting| posting.account == "Assets:Invest:Broker:Galaxy:Cash");
    assert!(has_expected_cash_account);
}

#[test]
fn uses_nested_securities_cash_account_when_configured() {
    let options = SecurityTransformOptions {
        provider_name: "yinhe",
        default_payee: "Galaxy",
    };
    let config = ProviderConfig {
        default_asset_account: Some("Assets:Broker:Galaxy:Securities".to_string()),
        default_cash_account: Some("Assets:Legacy:Broker:Cash".to_string()),
        securities_accounts: SecuritiesAccountsConfig {
            cash_account: Some("Assets:Nested:Broker:Cash".to_string()),
            ..SecuritiesAccountsConfig::default()
        },
        ..ProviderConfig::default()
    };

    let tx = build_security_trade_transaction(
        options,
        &MatchResult::default(),
        &config,
        make_context(dec!(1000), "buy", "159915", dec!(100), Some(dec!(10))),
    )
    .expect("trade should build");

    let has_expected_cash_account = tx
        .postings
        .iter()
        .any(|posting| posting.account == "Assets:Nested:Broker:Cash");
    assert!(has_expected_cash_account);
}

#[test]
fn uses_explicit_repo_interest_account_when_configured() {
    let options = SecurityTransformOptions {
        provider_name: "yinhe",
        default_payee: "Galaxy",
    };
    let config = ProviderConfig {
        default_asset_account: Some("Assets:Broker:Galaxy:Securities".to_string()),
        default_cash_account: Some("Assets:Broker:Galaxy:Cash".to_string()),
        default_repo_interest_account: Some("Income:Broker:Galaxy:RepoInterest".to_string()),
        ..ProviderConfig::default()
    };

    let tx = build_security_trade_transaction(
        options,
        &MatchResult::default(),
        &config,
        make_context(dec!(1010), "sell", "204001", dec!(10), None),
    )
    .expect("repo trade should build");

    let has_expected_interest_account = tx
        .postings
        .iter()
        .any(|posting| posting.account == "Income:Broker:Galaxy:RepoInterest");
    assert!(has_expected_interest_account);
}
