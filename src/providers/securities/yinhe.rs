//! 银河证券 Provider.

use crate::{
    error::{ImporterError, ImporterResult},
    interface::provider::Provider,
    model::{
        account::{amount::Amount, posting::Posting},
        config::provider::ProviderConfig,
        data::raw_record::RawRecord,
        rule::rule_engine::RuleEngine,
        transaction::Transaction,
    },
    providers::shared::{
        SecurityTransformOptions, append_extra_metadata, append_order_id, apply_match_result,
        transform_security_record,
    },
};

const YINHE_OPTIONS: SecurityTransformOptions = SecurityTransformOptions {
    provider_name: "yinhe",
    default_payee: "Galaxy",
};

const YINHE_INTEREST_ROLLOVER_KEYWORD: &str = "利息归本";
const YINHE_REPO_SETTLEMENT_KEYWORD: &str = "债券质押回购融券清算";
const YINHE_REPO_MATURE_SETTLEMENT_KEYWORD: &str = "债券质押回购融券到期清算";
const NORMALIZED_REPO_SELL_TYPE: &str = "融券购回";

/// Returns true when `record.symbol` is present and not blank.
fn has_non_empty_symbol(record: &RawRecord) -> bool {
    record
        .symbol
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
}

/// Detects Yinhe "利息归本" records that do not carry a security symbol.
/// These records are handled as cash-interest movements.
fn is_interest_rollover_without_symbol(record: &RawRecord) -> bool {
    let is_interest_rollover = record
        .transaction_type
        .as_deref()
        .map(|value| value.contains(YINHE_INTEREST_ROLLOVER_KEYWORD))
        .unwrap_or(false);

    is_interest_rollover && !has_non_empty_symbol(record)
}

/// Normalizes Yinhe-specific transaction type variants to shared canonical values.
fn normalize_yinhe_record(mut record: RawRecord) -> RawRecord {
    if record
        .transaction_type
        .as_deref()
        .map(|value| {
            value.contains(YINHE_REPO_SETTLEMENT_KEYWORD)
                || value.contains(YINHE_REPO_MATURE_SETTLEMENT_KEYWORD)
        })
        .unwrap_or(false)
    {
        record.transaction_type = Some(NORMALIZED_REPO_SELL_TYPE.to_string());
    }

    record
}

/// Normalizes Yinhe cash currency labels to ISO-style currency codes.
fn normalize_cash_currency_for_yinhe(raw: Option<&str>) -> String {
    let trimmed = raw.unwrap_or("CNY").trim();
    if trimmed.is_empty() {
        return "CNY".to_string();
    }

    match trimmed {
        "人民币" | "人民币元" | "RMB" | "CNY" => "CNY".to_string(),
        "美元" | "USD" => "USD".to_string(),
        "港币" | "港元" | "HKD" => "HKD".to_string(),
        "欧元" | "EUR" => "EUR".to_string(),
        "英镑" | "GBP" => "GBP".to_string(),
        "日元" | "JPY" => "JPY".to_string(),
        _ => trimmed.to_ascii_uppercase(),
    }
}

/// Derives a broker cash account from `default_asset_account` when not explicitly set.
fn derive_cash_account_for_yinhe(default_asset_account: Option<&str>) -> String {
    if let Some(account) = default_asset_account.map(str::trim) {
        if account.ends_with(":Cash") || account.ends_with(":人民币资产") {
            return account.to_string();
        }
        if let Some(prefix) = account.strip_suffix(":Securities") {
            return format!("{}:Cash", prefix);
        }
        if let Some(prefix) = account.strip_suffix(":证券资产") {
            return format!("{}:人民币资产", prefix);
        }
    }

    "Assets:Broker:Cash".to_string()
}

/// Resolves broker cash account with `default_cash_account` taking highest priority.
fn resolve_broker_cash_account(config: &ProviderConfig) -> String {
    config
        .default_cash_account
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| derive_cash_account_for_yinhe(config.default_asset_account.as_deref()))
}

/// Resolves account used for positive "利息归本" postings.
fn resolve_interest_account(config: &ProviderConfig) -> String {
    config
        .default_repo_interest_account
        .clone()
        .or(config.default_income_account.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Income:Investing:Interest".to_string())
}

/// Resolves account used for negative "利息归本" postings.
fn resolve_fee_account(config: &ProviderConfig) -> String {
    config
        .default_fee_account
        .clone()
        .or(config.default_expense_account.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Expenses:Investing:Fees".to_string())
}

/// Builds a transaction for Yinhe "利息归本" rows without symbol.
/// Positive amount credits interest; negative amount treats it as fee/adjustment.
fn build_yinhe_interest_rollover_transaction(
    mut record: RawRecord,
    rule_engine: &RuleEngine,
    config: &ProviderConfig,
) -> ImporterResult<Option<Transaction>> {
    let match_result = rule_engine.match_record(&record);
    if match_result.ignore {
        return Ok(None);
    }

    let date = record
        .date
        .ok_or_else(|| ImporterError::Conversion("Missing trade date".to_string()))?;
    let amount = record.amount.ok_or_else(|| {
        ImporterError::Conversion("Missing amount for interest rollover".to_string())
    })?;

    let currency = normalize_cash_currency_for_yinhe(
        record
            .currency
            .as_deref()
            .or(config.default_currency.as_deref()),
    );
    let broker_cash_account = resolve_broker_cash_account(config);
    let interest_account = resolve_interest_account(config);
    let fee_account = resolve_fee_account(config);

    let tx_type_text = record
        .transaction_type
        .clone()
        .unwrap_or_else(|| YINHE_INTEREST_ROLLOVER_KEYWORD.to_string());
    let narration = match_result
        .narration
        .clone()
        .or(record.narration.clone())
        .unwrap_or(tx_type_text);

    let amount_abs = amount.abs();
    // Keep direction explicit so one path handles both income and adjustments.
    let (debit_account, credit_account) = if amount.is_sign_positive() {
        (
            match_result
                .debit_account
                .clone()
                .unwrap_or_else(|| broker_cash_account.clone()),
            match_result
                .credit_account
                .clone()
                .unwrap_or_else(|| interest_account),
        )
    } else {
        (
            match_result
                .debit_account
                .clone()
                .unwrap_or_else(|| fee_account),
            match_result
                .credit_account
                .clone()
                .unwrap_or_else(|| broker_cash_account.clone()),
        )
    };

    let mut tx = Transaction::new(date, narration)
        .with_posting(
            Posting::new(debit_account).with_amount(Amount::new(amount_abs, currency.clone())),
        )
        .with_posting(Posting::new(credit_account).with_amount(Amount::new(-amount_abs, currency)));

    tx = append_order_id(tx, "yinhe", record.reference.take());
    tx = append_extra_metadata(tx, "yinhe", record.extra);
    tx = apply_match_result(
        tx,
        "yinhe",
        &match_result,
        record
            .payee
            .or_else(|| Some(YINHE_OPTIONS.default_payee.to_string())),
        config.name.as_deref(),
    );

    Ok(Some(tx))
}

pub struct YinheProvider;

impl Provider for YinheProvider {
    fn name(&self) -> &'static str {
        "yinhe"
    }

    fn description(&self) -> &'static str {
        "Yinhe securities statement importer"
    }

    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        if is_interest_rollover_without_symbol(&record) {
            return build_yinhe_interest_rollover_transaction(record, rule_engine, config);
        }

        let record = normalize_yinhe_record(record);
        transform_security_record(YINHE_OPTIONS, record, rule_engine, config)
    }
}

#[cfg(test)]
mod tests {
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

        let mut config = ProviderConfig::default();
        config.default_cash_account = Some("Assets:Broker:Galaxy:Cash".to_string());
        config.default_repo_interest_account =
            Some("Income:Broker:Galaxy:RepoInterest".to_string());

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
}
