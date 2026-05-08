//! 银河证券（Yinhe）账单导入适配器。
//!
//! 在复用共享证券转换流水线前，本模块额外处理两类银河特化语义：
//! - 将“债券质押回购融券清算/到期清算”归一化为统一交易类型；
//! - 将“利息归本 + 无证券代码”识别为现金利息流转，并直接构建双分录交易。
//!
//! 其余记录保持通用转换逻辑，避免 Provider 之间重复实现。
//!
//! # 示例
//! ```rust,no_run
//! use beancount_importer_rust::{
//!     interface::provider::Provider,
//!     model::{
//!         config::{global::GlobalConfig, provider::ProviderConfig},
//!         data::raw_record::RawRecord,
//!         rule::{Rule, rule_engine::RuleEngine},
//!     },
//!     providers::securities::yinhe::YinheProvider,
//! };
//!
//! let provider = YinheProvider;
//! assert_eq!(provider.name(), "yinhe");
//! assert_eq!(provider.description(), "Yinhe securities statement importer");
//!
//! let config = ProviderConfig::default();
//! let global = GlobalConfig::default();
//! let provider_rules: [Rule; 0] = [];
//! let rule_engine = RuleEngine::new(&provider_rules, &global);
//! let record = RawRecord::new();
//!
//! let _ = provider.transform(record, &rule_engine, &config)?;
//! # Ok::<(), beancount_importer_rust::error::ImporterError>(())
//! ```

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
    utils::currency::normalize_cash_currency,
};

/// 银河证券共享转换参数。
const YINHE_OPTIONS: SecurityTransformOptions = SecurityTransformOptions {
    default_payee: "Galaxy",
};

/// 银河“利息归本”交易类型关键字。
const YINHE_INTEREST_ROLLOVER_KEYWORD: &str = "利息归本";
/// 银河“逆回购清算”交易类型关键字。
const YINHE_REPO_SETTLEMENT_KEYWORD: &str = "债券质押回购融券清算";
/// 银河“逆回购到期清算”交易类型关键字。
const YINHE_REPO_MATURE_SETTLEMENT_KEYWORD: &str = "债券质押回购融券到期清算";
/// 共享转换层可识别的逆回购卖出类型。
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

/// 未显式配置现金账户时，从 `default_asset_account` 推导券商现金账户。
///
/// 兼容两类账户命名：
/// - `...:Securities` -> `...:Cash`
/// - `...:证券资产` -> `...:人民币资产`
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

/// 解析券商现金账户：优先使用 `securities_accounts.cash_account`。
///
/// 若配置缺失则回退到 [`derive_cash_account_for_yinhe`] 结果。
fn resolve_broker_cash_account(config: &ProviderConfig) -> String {
    config
        .securities_cash_account()
        .map(str::to_string)
        .unwrap_or_else(|| derive_cash_account_for_yinhe(config.default_asset_account.as_deref()))
}

/// 解析“利息归本”为正金额时使用的收益账户。
///
/// 优先级：`securities_repo_interest_account` -> `default_income_account` -> 默认常量。
fn resolve_interest_account(config: &ProviderConfig) -> String {
    config
        .securities_repo_interest_account()
        .map(str::to_string)
        .or(config.default_income_account.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Income:Investing:Interest".to_string())
}

/// 解析“利息归本”为负金额时使用的费用账户。
///
/// 优先级：`securities_fee_account` -> `default_expense_account` -> 默认常量。
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
///
/// # 返回值
/// - `Ok(Some(Transaction))`：成功构建交易。
/// - `Ok(None)`：命中规则引擎忽略条件。
///
/// # Errors
/// 当记录缺失 `date` 或 `amount` 时返回 `ImporterError::Conversion`。
fn build_yinhe_interest_rollover_transaction(
    provider_name: &str,
    display_name: &str,
    mut record: RawRecord,
    rule_engine: &RuleEngine,
    config: &ProviderConfig,
) -> ImporterResult<Option<Transaction>> {
    let match_result = rule_engine.match_record(&record);
    if match_result.ignore {
        return Ok(None);
    }

    // 利息归本属于资金变动，缺少交易日或金额无法安全入账。
    let date = record
        .date
        .ok_or_else(|| ImporterError::Conversion("Missing trade date".to_string()))?;
    let amount = record.amount.ok_or_else(|| {
        ImporterError::Conversion("Missing amount for interest rollover".to_string())
    })?;

    let currency = normalize_cash_currency(
        record
            .currency
            .as_deref()
            .or(config.default_currency.as_deref()),
    );
    let broker_cash_account = resolve_broker_cash_account(config);
    let interest_account = resolve_interest_account(config);
    let fee_account = resolve_fee_account(config);

    // narration 回退顺序：规则覆盖 -> 原始摘要 -> 交易类型文本。
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
    // 明确正负方向，让同一逻辑同时覆盖收益与冲减场景：
    // - 正数：借券商现金，贷利息收入；
    // - 负数：借费用（或冲减），贷券商现金。
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

    // 对齐共享转换产物，统一补充订单号、扩展元数据与规则引擎动作结果。
    tx = append_order_id(tx, provider_name, record.reference.take());
    tx = append_extra_metadata(tx, provider_name, record.extra);
    let source_label = config.name.as_deref().unwrap_or(display_name);
    tx = apply_match_result(tx, provider_name, &match_result, record.payee.or_else(|| Some(YINHE_OPTIONS.default_payee.to_string())), source_label);

    Ok(Some(tx))
}

/// 银河证券账单 `Provider`。
///
/// 使用无状态零大小类型（ZST）实现，仅承担银河特化语义分派职责。
pub struct YinheProvider;

impl Provider for YinheProvider {
    /// 返回供应商唯一标识：`"yinhe"`。
    fn name(&self) -> &'static str {
        "yinhe"
    }

    /// 返回供应商描述信息。
    fn description(&self) -> &'static str {
        "Yinhe securities statement importer"
    }

    fn display_name(&self) -> &'static str {
        "银河证券"
    }

    /// 将一条银河原始记录转换为交易。
    ///
    /// 路由规则：
    /// 1. “利息归本 + 无证券代码” -> 专用利息交易构建逻辑；
    /// 2. 其他记录 -> 银河交易类型归一化后进入共享证券转换层。
    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        if is_interest_rollover_without_symbol(&record) {
            return build_yinhe_interest_rollover_transaction(
                self.name(),
                self.display_name(),
                record,
                rule_engine,
                config,
            );
        }

        // 常规证券流水在进入共享层前做一次银河术语归一化。
        let record = normalize_yinhe_record(record);
        transform_security_record(self.name(), self.display_name(), YINHE_OPTIONS, record, rule_engine, config)
    }
}

#[cfg(test)]
mod tests;
