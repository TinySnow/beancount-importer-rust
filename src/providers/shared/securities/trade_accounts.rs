use crate::model::{config::provider::ProviderConfig, rule::match_result::MatchResult};

use super::logic::{derive_cash_account, derive_rounding_account};

/// 证券交易所需账户集合。
#[derive(Debug)]
pub(super) struct TradeAccountPlan {
    pub(super) holdings_account: String,
    pub(super) cash_account: String,
    pub(super) fee_account: String,
    pub(super) rounding_account: String,
    pub(super) pnl_account: String,
    pub(super) interest_account: String,
}

/// 按交易方向与配置解析证券交易涉及账户。
pub(super) fn build_trade_account_plan(
    match_result: &MatchResult,
    config: &ProviderConfig,
    is_buy: bool,
) -> TradeAccountPlan {
    let holdings_account = if is_buy {
        match_result
            .debit_account
            .clone()
            .or(config.default_asset_account.clone())
            .unwrap_or_else(|| "Assets:Investments".to_string())
    } else {
        match_result
            .credit_account
            .clone()
            .or(config.default_asset_account.clone())
            .unwrap_or_else(|| "Assets:Investments".to_string())
    };

    let broker_cash_account = config
        .securities_cash_account()
        .map(str::to_string)
        .unwrap_or_else(|| derive_cash_account(config.default_asset_account.as_deref()));

    let cash_account = if is_buy {
        match_result
            .credit_account
            .clone()
            .unwrap_or_else(|| broker_cash_account.clone())
    } else {
        match_result
            .debit_account
            .clone()
            .unwrap_or_else(|| broker_cash_account.clone())
    };

    let fee_account = match_result
        .fee_account
        .clone()
        .or_else(|| config.securities_fee_account().map(str::to_string))
        .or(config.default_expense_account.clone())
        .unwrap_or_else(|| "Expenses:Investing:Fees".to_string());

    let rounding_account = match_result
        .rounding_account
        .clone()
        .or_else(|| config.securities_rounding_account().map(str::to_string))
        .unwrap_or_else(|| derive_rounding_account(&fee_account));

    let pnl_account = match_result
        .pnl_account
        .clone()
        .or_else(|| config.securities_pnl_account().map(str::to_string))
        .or(config.default_income_account.clone())
        .filter(|value| value != "Income:Unknown")
        .unwrap_or_else(|| "Income:Investing:Capital-Gains".to_string());

    let interest_account = config
        .securities_repo_interest_account()
        .map(str::to_string)
        .unwrap_or_else(|| "Income:Investing:Interest".to_string());

    TradeAccountPlan {
        holdings_account,
        cash_account,
        fee_account,
        rounding_account,
        pnl_account,
        interest_account,
    }
}
