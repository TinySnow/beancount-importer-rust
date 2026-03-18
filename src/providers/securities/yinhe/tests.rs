use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::model::{
    config::{global::GlobalConfig, provider::ProviderConfig},
    data::raw_record::RawRecord,
    rule::{Rule, rule_engine::RuleEngine},
};

use super::{
    build_yinhe_interest_rollover_transaction, is_interest_rollover_without_symbol,
    normalize_yinhe_record,
};

#[test]
fn recognizes_interest_rollover_without_symbol() {
    let mut record = RawRecord::new();
    record.transaction_type = Some("利息归本".to_string());
    record.symbol = Some("   ".to_string());

    assert!(is_interest_rollover_without_symbol(&record));
}

#[test]
fn keeps_interest_rollover_when_symbol_present() {
    let mut record = RawRecord::new();
    record.transaction_type = Some("利息归本".to_string());
    record.symbol = Some("131810".to_string());

    assert!(!is_interest_rollover_without_symbol(&record));
}

#[test]
fn normalizes_repo_settlement_transaction_type() {
    let mut record = RawRecord::new();
    record.transaction_type = Some("债券质押回购融券清算".to_string());

    let normalized = normalize_yinhe_record(record);
    assert_eq!(normalized.transaction_type.as_deref(), Some("融券购回"));
}

#[test]
fn normalizes_repo_mature_settlement_transaction_type() {
    let mut record = RawRecord::new();
    record.transaction_type = Some("债券质押回购融券到期清算".to_string());

    let normalized = normalize_yinhe_record(record);
    assert_eq!(normalized.transaction_type.as_deref(), Some("融券购回"));
}

#[test]
fn builds_interest_rollover_transaction_into_interest_account() {
    let mut record = RawRecord::new();
    record.date = NaiveDate::from_ymd_opt(2026, 2, 1);
    record.amount = Some(Decimal::new(1234, 2));
    record.currency = Some("CNY".to_string());
    record.transaction_type = Some("利息归本".to_string());
    record.reference = Some("order-1".to_string());
    record.payee = Some("银河证券".to_string());
    record
        .extra
        .insert("txType".to_string(), "利息归本".to_string());

    let config = ProviderConfig {
        default_cash_account: Some("Assets:Broker:Galaxy:Cash".to_string()),
        default_repo_interest_account: Some("Income:Broker:Galaxy:RepoInterest".to_string()),
        ..ProviderConfig::default()
    };

    let provider_rules: &'static [Rule] = Box::leak(Vec::<Rule>::new().into_boxed_slice());
    let global: &'static GlobalConfig = Box::leak(Box::new(GlobalConfig::default()));
    let rule_engine = RuleEngine::new(provider_rules, global);

    let tx = build_yinhe_interest_rollover_transaction(record, &rule_engine, &config)
        .expect("interest rollover should build")
        .expect("interest rollover should not be ignored");

    assert_eq!(tx.postings.len(), 2);
    assert_eq!(tx.postings[0].account, "Assets:Broker:Galaxy:Cash");
    assert_eq!(tx.postings[1].account, "Income:Broker:Galaxy:RepoInterest");
    assert_eq!(
        tx.postings[0].amount.as_ref().map(|value| value.number),
        Some(Decimal::new(1234, 2))
    );
    assert_eq!(
        tx.postings[1].amount.as_ref().map(|value| value.number),
        Some(Decimal::new(-1234, 2))
    );
}
