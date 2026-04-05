//! 现金流原始记录到交易对象的编排入口。
//!
//! 该模块负责把来自不同 Provider 的现金流水记录转换为统一的
//! `Transaction`：先做规则匹配与字段归一，再根据收支方向选择账户并构建双分录。

use crate::{
    error::{ImporterError, ImporterResult},
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        rule::rule_engine::RuleEngine, transaction::Transaction,
    },
    providers::shared::{append_extra_metadata, append_order_id, apply_match_result},
    utils::currency::normalize_cash_currency,
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
///
/// 当缺少必要字段时返回 `ImporterError::Conversion`，调用方可据此决定是否记录失败项。
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

    let currency =
        normalize_cash_currency(currency.as_deref().or(config.default_currency.as_deref()));

    let narration = match_result
        .narration
        .clone()
        .or(narration)
        .unwrap_or_else(|| "Unknown transaction".to_string());

    // 优先使用结构化字段 `transaction_type`，其次读取原始扩展字段 `extra.type`。
    let direction = transaction_type
        .as_deref()
        .map(str::to_string)
        .or_else(|| extra.get("type").cloned());

    let is_expense = infer_is_expense(direction.as_deref(), amount);

    let mut tx = Transaction::new(date, narration);

    if is_expense {
        // 支出: 借费用、贷资产。规则账户优先于配置默认值。
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
        // 收入: 借资产、贷收入。规则账户优先于配置默认值。
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

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    use crate::model::{
        config::{global::GlobalConfig, provider::ProviderConfig},
        data::raw_record::RawRecord,
        rule::{Rule, rule_engine::RuleEngine},
    };

    use super::{CashflowTransformOptions, transform_cashflow_record};

    const TEST_OPTIONS: CashflowTransformOptions = CashflowTransformOptions {
        provider_name: "ccb",
        default_asset_fallback: "Assets:CCB",
    };

    #[test]
    fn normalizes_rmb_currency_from_record_to_cny() {
        let mut record = RawRecord::new();
        record.date = NaiveDate::from_ymd_opt(2026, 3, 6);
        record.amount = Some(dec!(2000));
        record.currency = Some("人民币".to_string());
        record.transaction_type = Some("支出".to_string());
        record.narration = Some("基金下账".to_string());

        let config = ProviderConfig::default();
        let global = GlobalConfig::default();
        let provider_rules: [Rule; 0] = [];
        let rule_engine = RuleEngine::new(&provider_rules, &global);

        let tx = transform_cashflow_record(TEST_OPTIONS, record, &rule_engine, &config)
            .expect("cashflow transform should succeed")
            .expect("record should not be ignored");

        let currencies: Vec<String> = tx
            .postings
            .iter()
            .filter_map(|posting| {
                posting
                    .amount
                    .as_ref()
                    .map(|amount| amount.currency.clone())
            })
            .collect();

        assert_eq!(currencies, vec!["CNY".to_string(), "CNY".to_string()]);
    }

    #[test]
    fn normalizes_rmb_default_currency_when_record_currency_missing() {
        let mut record = RawRecord::new();
        record.date = NaiveDate::from_ymd_opt(2026, 3, 6);
        record.amount = Some(dec!(10));
        record.transaction_type = Some("收入".to_string());
        record.narration = Some("退款".to_string());

        let config = ProviderConfig {
            default_currency: Some("人民币".to_string()),
            ..ProviderConfig::default()
        };
        let global = GlobalConfig::default();
        let provider_rules: [Rule; 0] = [];
        let rule_engine = RuleEngine::new(&provider_rules, &global);

        let tx = transform_cashflow_record(TEST_OPTIONS, record, &rule_engine, &config)
            .expect("cashflow transform should succeed")
            .expect("record should not be ignored");

        let currencies: Vec<String> = tx
            .postings
            .iter()
            .filter_map(|posting| {
                posting
                    .amount
                    .as_ref()
                    .map(|amount| amount.currency.clone())
            })
            .collect();

        assert_eq!(currencies, vec!["CNY".to_string(), "CNY".to_string()]);
    }

    #[test]
    fn normalizes_chinese_usd_label_to_usd() {
        let mut record = RawRecord::new();
        record.date = NaiveDate::from_ymd_opt(2026, 3, 6);
        record.amount = Some(dec!(100));
        record.currency = Some("美元".to_string());
        record.transaction_type = Some("支出".to_string());
        record.narration = Some("外币消费".to_string());

        let config = ProviderConfig::default();
        let global = GlobalConfig::default();
        let provider_rules: [Rule; 0] = [];
        let rule_engine = RuleEngine::new(&provider_rules, &global);

        let tx = transform_cashflow_record(TEST_OPTIONS, record, &rule_engine, &config)
            .expect("cashflow transform should succeed")
            .expect("record should not be ignored");

        let currencies: Vec<String> = tx
            .postings
            .iter()
            .filter_map(|posting| {
                posting
                    .amount
                    .as_ref()
                    .map(|amount| amount.currency.clone())
            })
            .collect();

        assert_eq!(currencies, vec!["USD".to_string(), "USD".to_string()]);
    }
}
