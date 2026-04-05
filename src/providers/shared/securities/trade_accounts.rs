//! 证券交易账户规划逻辑。
//!
//! 根据规则匹配结果与 Provider 默认配置，生成交易构建所需的账户组合。

use crate::model::{config::provider::ProviderConfig, rule::match_result::MatchResult};

use super::logic::{derive_cash_account, derive_rounding_account};

/// 证券交易所需账户集合。
#[derive(Debug)]
pub(super) struct TradeAccountPlan {
    /// 证券持仓账户。
    pub(super) holdings_account: String,
    /// 券商现金账户（交易对手现金腿）。
    pub(super) cash_account: String,
    /// 手续费账户。
    pub(super) fee_account: String,
    /// 舍入差异账户。
    pub(super) rounding_account: String,
    /// 已实现盈亏账户。
    pub(super) pnl_account: String,
    /// 逆回购利息账户。
    pub(super) interest_account: String,
}

/// 按交易方向与配置解析证券交易涉及账户。
///
/// 账户优先级总体遵循：
/// 1. 规则显式指定；
/// 2. Provider 配置默认值；
/// 3. 模块内置兜底账户。
pub(super) fn build_trade_account_plan(
    match_result: &MatchResult,
    config: &ProviderConfig,
    is_buy: bool,
) -> TradeAccountPlan {
    // 持仓账户在买卖方向上对应不同的规则字段。
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

    // 买入时现金腿通常在贷方，卖出时在借方。
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
