//! 模块说明：跨 Provider 的现金流分类与分录构建能力。
//!
//! 文件路径：src/providers/shared/cashflow/transform.rs。
//! 该文件聚焦原始记录到交易的转换编排。
//! 关键符号：无显式公开符号，主要通过内部实现或模块组织发挥作用。

use crate::{
    error::{ImporterError, ImporterResult},
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        rule::rule_engine::RuleEngine, transaction::Transaction,
    },
    providers::shared::{append_extra_metadata, append_order_id, apply_match_result},
};

use super::{
    CashflowTransformOptions,
    classify::infer_is_expense,
    posting::{apply_expense_postings, apply_income_postings},
};

/// 银行/钱包/第三方支付类供应商的通用现金流转换入口。
///
/// 处理流程：
/// 1. 执行规则引擎并处理 `ignore`。
/// 2. 解析必要字段（日期、金额、币种）。
/// 3. 判定收支方向并构建分录。
/// 4. 附加订单号、扩展字段与规则输出元数据。
pub(crate) fn transform_cashflow_record(
    options: CashflowTransformOptions,
    record: RawRecord,
    rule_engine: &RuleEngine,
    config: &ProviderConfig,
) -> ImporterResult<Option<Transaction>> {
    let match_result = rule_engine.match_record(&record);
    if match_result.ignore {
        return Ok(None);
    }

    let RawRecord {
        date,
        amount,
        currency,
        payee,
        narration,
        transaction_type,
        reference,
        extra,
        ..
    } = record;

    let date = date.ok_or_else(|| ImporterError::Conversion("Missing date".to_string()))?;
    let amount = amount.ok_or_else(|| ImporterError::Conversion("Missing amount".to_string()))?;

    let currency = currency
        .or(config.default_currency.clone())
        .unwrap_or_else(|| "CNY".to_string());

    let narration = match_result
        .narration
        .clone()
        .or(narration)
        .unwrap_or_else(|| "Unknown transaction".to_string());

    let direction = transaction_type
        .as_deref()
        .map(str::to_string)
        .or_else(|| extra.get("type").cloned());

    let is_expense = infer_is_expense(direction.as_deref(), amount);

    let mut tx = Transaction::new(date, narration);

    if is_expense {
        let expense_account = match_result
            .debit_account
            .clone()
            .or(config.default_expense_account.clone())
            .unwrap_or_else(|| "Expenses:Unknown".to_string());

        let asset_account = match_result
            .credit_account
            .clone()
            .or(config.default_asset_account.clone())
            .unwrap_or_else(|| options.default_asset_fallback.to_string());

        tx = apply_expense_postings(tx, &expense_account, &asset_account, amount, &currency);
    } else {
        let income_account = match_result
            .credit_account
            .clone()
            .or(config.default_income_account.clone())
            .unwrap_or_else(|| "Income:Unknown".to_string());

        let asset_account = match_result
            .debit_account
            .clone()
            .or(config.default_asset_account.clone())
            .unwrap_or_else(|| options.default_asset_fallback.to_string());

        tx = apply_income_postings(tx, &asset_account, &income_account, amount, &currency);
    }

    tx = append_order_id(tx, options.provider_name, reference);
    tx = append_extra_metadata(tx, options.provider_name, extra);
    tx = apply_match_result(
        tx,
        options.provider_name,
        &match_result,
        payee,
        config.name.as_deref(),
    );

    Ok(Some(tx))
}
