//! 逆回购交易分录构建。
//!
//! 与普通现货交易不同，逆回购使用固定面值建模持仓成本，
//! 并把本金差额解释为利息或费用。

use rust_decimal::Decimal;

use crate::model::{
    account::{amount::Amount, cost::Cost, posting::Posting},
    transaction::Transaction,
};

use super::super::{
    REPO_FACE_VALUE,
    posting::{append_buy_fee_or_rounding, append_repo_interest_or_loss},
};

/// 逆回购分录构建输入。
pub(super) struct RepoPostingInput<'a> {
    /// 待追加分录的交易对象。
    pub(super) tx: Transaction,
    /// 逆回购持仓账户。
    pub(super) holdings_account: &'a str,
    /// 券商现金账户。
    pub(super) cash_account: &'a str,
    /// 商品代码（已归一化）。
    pub(super) commodity_symbol: &'a str,
    /// 现金币种。
    pub(super) cash_currency: &'a str,
    /// 带方向的持仓数量（买入为正、卖出为负）。
    pub(super) signed_quantity: Decimal,
    /// 带方向的现金金额（买入为负、卖出为正）。
    pub(super) signed_cash: Decimal,
    /// 原始数量绝对值（用于本金计算）。
    pub(super) quantity: Decimal,
    /// 原始现金金额绝对值（用于差额计算）。
    pub(super) cash_amount: Decimal,
    /// 是否买入方向。
    pub(super) is_buy: bool,
    /// 手续费账户。
    pub(super) fee_account: &'a str,
    /// 舍入差异账户。
    pub(super) rounding_account: &'a str,
    /// 逆回购利息收入账户。
    pub(super) interest_account: &'a str,
}

/// 应用逆回购持仓与现金分录。
///
/// 规则：
/// - 以固定面值（100 CNY）记录持仓成本。
/// - 现金与本金差额在买入侧记手续费/舍入，卖出侧记利息/费用。
pub(super) fn apply_repo_postings(input: RepoPostingInput<'_>) -> Transaction {
    let RepoPostingInput {
        mut tx,
        holdings_account,
        cash_account,
        commodity_symbol,
        cash_currency,
        signed_quantity,
        signed_cash,
        quantity,
        cash_amount,
        is_buy,
        fee_account,
        rounding_account,
        interest_account,
    } = input;

    tx = tx.with_posting(
        Posting::new(holdings_account)
            .with_amount(Amount::new(signed_quantity, commodity_symbol.to_string()))
            .with_cost(Cost::new(
                Decimal::from(REPO_FACE_VALUE),
                cash_currency.to_string(),
            )),
    );

    tx = tx.with_posting(
        Posting::new(cash_account).with_amount(Amount::new(signed_cash, cash_currency.to_string())),
    );

    // 理论本金 = 份额 * 面值。实际现金与理论本金的差即费用/利息来源。
    let principal = quantity.abs() * Decimal::from(REPO_FACE_VALUE);
    let delta = cash_amount - principal;

    if is_buy {
        append_buy_fee_or_rounding(tx, delta, cash_currency, fee_account, rounding_account)
    } else {
        append_repo_interest_or_loss(tx, delta, cash_currency, interest_account, fee_account)
    }
}
