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

/// 构建银证转账类交易。
///
/// 适用场景：账单记录中不存在证券代码，仅表示资金在银行与券商现金账户之间划转。
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

    // 券商现金账户优先从 default_asset_account 推导，避免配置重复。
    let broker_cash_account = derive_cash_account(config.default_asset_account.as_deref());
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

    tx = tx.with_posting(Posting::new(debit_account).with_amount(Amount::new(
        transfer_amount.abs(),
        cash_currency.to_string(),
    )));
    tx = tx.with_posting(Posting::new(credit_account).with_amount(Amount::new(
        -transfer_amount.abs(),
        cash_currency.to_string(),
    )));

    // 供应商原始费税信息保留在 metadata，便于审计追溯。
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
