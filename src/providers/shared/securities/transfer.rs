//! 银证现金划转分录构建。
//!
//! 负责“银行 <-> 券商”纯现金划转场景，不涉及证券持仓变动。

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
    DEFAULT_TRANSFER_ASSET_ACCOUNT,
    context::SecurityRecordContext,
    logic::{Direction, derive_cash_account, infer_transfer_direction},
};

/// 构建“券商 <-> 银行”现金划转交易。
///
/// 划转方向可被规则中的 `debit_account` / `credit_account` 覆盖。
/// 若规则未覆盖，则使用 Provider 配置或模块默认账户。
pub(super) fn build_cash_transfer_transaction(
    provider_name: &str,
    display_name: &str,
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
        fee,
        tax,
        extra,
        ..
    } = context;

    let transfer_amount = amount.ok_or_else(|| {
        ImporterError::Conversion("Missing transfer amount for cash transfer".to_string())
    })?;
    let direction = infer_transfer_direction(transaction_type.as_deref(), transfer_amount);

    // 券商现金账户优先读取配置，缺省时再从默认资产账户推导。
    let broker_cash_account = config
        .securities_cash_account()
        .map(str::to_string)
        .unwrap_or_else(|| derive_cash_account(config.default_asset_account.as_deref()));

    // 先使用规则覆盖账户，再回落到 provider 默认账户。
    let (debit_account, credit_account) = if direction == Direction::In {
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

    // 统一符号语义：借方为正，贷方为负。
    tx = tx.with_posting(
        Posting::new(debit_account)
            .with_amount(Amount::new(transfer_amount.abs(), cash_currency.clone())),
    );
    tx = tx.with_posting(
        Posting::new(credit_account)
            .with_amount(Amount::new(-transfer_amount.abs(), cash_currency)),
    );

    if let Some(fee) = fee {
        tx = tx.with_meta("fee", MetaValue::Number(fee));
    }
    if let Some(tax) = tax {
        tx = tx.with_meta("tax", MetaValue::Number(tax));
    }

    tx = append_order_id(tx, provider_name, reference);
    tx = append_extra_metadata(tx, provider_name, extra);
    tx = apply_match_result(tx, provider_name, match_result, payee.or(transaction_type), display_name);

    Ok(tx)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    use super::super::context::SecurityRecordContext;
    use super::build_cash_transfer_transaction;
    use crate::model::{
        config::provider::{ProviderConfig, SecuritiesAccountsConfig},
        rule::match_result::MatchResult,
    };

    #[test]
    fn uses_explicit_securities_cash_account_for_cash_transfer() {
        let config = ProviderConfig {
            default_asset_account: Some("Assets:Broker:Galaxy:Securities".to_string()),
            securities_accounts: SecuritiesAccountsConfig {
                cash_account: Some("Assets:Invest:Broker:Galaxy:Cash".to_string()),
                ..SecuritiesAccountsConfig::default()
            },
            ..ProviderConfig::default()
        };

        let context = SecurityRecordContext {
            date: NaiveDate::from_ymd_opt(2026, 3, 1).expect("valid date"),
            amount: Some(dec!(1000)),
            cash_currency: "CNY".to_string(),
            payee: None,
            narration: None,
            transaction_type: Some("in".to_string()),
            reference: None,
            symbol: None,
            security_name: None,
            quantity: None,
            unit_price: None,
            fee: None,
            tax: None,
            extra: HashMap::new(),
        };

        let tx =
            build_cash_transfer_transaction("yinhe", "银河证券", &MatchResult::default(), &config, context)
                .expect("cash transfer should build");

        let has_custom_cash_account = tx
            .postings
            .iter()
            .any(|posting| posting.account == "Assets:Invest:Broker:Galaxy:Cash");
        assert!(has_custom_cash_account);
    }
}
