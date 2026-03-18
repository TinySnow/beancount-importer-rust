//! 银河证券供应商实现。

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

/// 判断 `record.symbol` 是否存在且非空白。
fn has_non_empty_symbol(record: &RawRecord) -> bool {
    record
        .symbol
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
}

/// 识别银河“利息归本”且未携带证券代码的记录。
/// 这类记录按现金利息流转处理，而不是证券交易。
fn is_interest_rollover_without_symbol(record: &RawRecord) -> bool {
    let is_interest_rollover = record
        .transaction_type
        .as_deref()
        .map(|value| value.contains(YINHE_INTEREST_ROLLOVER_KEYWORD))
        .unwrap_or(false);

    is_interest_rollover && !has_non_empty_symbol(record)
}

/// 将银河特有交易类型归一化到共享语义值。
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

/// 归一化银河现金币种标识（转为统一币种代码）。
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

/// 未显式配置现金账户时，从 `default_asset_account` 推导券商现金账户。
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

/// 解析券商现金账户：优先使用 `default_cash_account`。
fn resolve_broker_cash_account(config: &ProviderConfig) -> String {
    config
        .securities_cash_account()
        .map(str::to_string)
        .unwrap_or_else(|| derive_cash_account_for_yinhe(config.default_asset_account.as_deref()))
}

/// 解析“利息归本”为正金额时使用的收益账户。
fn resolve_interest_account(config: &ProviderConfig) -> String {
    config
        .securities_repo_interest_account()
        .map(str::to_string)
        .or(config.default_income_account.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Income:Investing:Interest".to_string())
}

/// 解析“利息归本”为负金额时使用的费用账户。
fn resolve_fee_account(config: &ProviderConfig) -> String {
    config
        .securities_fee_account()
        .map(str::to_string)
        .or(config.default_expense_account.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Expenses:Investing:Fees".to_string())
}

/// 为银河“利息归本（无证券代码）”记录构建交易。
/// 金额为正记入收益，金额为负记入费用或冲正。
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
    // 明确正负方向，让同一逻辑同时覆盖收益与冲减场景。
    let (debit_account, credit_account) = if amount.is_sign_positive() {
        (
            match_result
                .debit_account
                .clone()
                .unwrap_or_else(|| broker_cash_account.clone()),
            match_result
                .credit_account
                .clone()
                .unwrap_or(interest_account),
        )
    } else {
        (
            match_result.debit_account.clone().unwrap_or(fee_account),
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
mod tests;
