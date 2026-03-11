use rust_decimal::Decimal;

use crate::model::{
    account::{amount::Amount, cost::Cost, posting::Posting},
    transaction::Transaction,
};

use super::{
    REPO_FACE_VALUE,
    posting::{append_buy_fee_or_rounding, append_repo_interest_or_loss},
};

/// 逆回购分录构建输入。
pub(super) struct RepoPostingInput<'a> {
    pub(super) tx: Transaction,
    pub(super) holdings_account: &'a str,
    pub(super) cash_account: &'a str,
    pub(super) commodity_symbol: &'a str,
    pub(super) cash_currency: &'a str,
    pub(super) signed_quantity: Decimal,
    pub(super) signed_cash: Decimal,
    pub(super) quantity: Decimal,
    pub(super) cash_amount: Decimal,
    pub(super) is_buy: bool,
    pub(super) fee_account: &'a str,
    pub(super) rounding_account: &'a str,
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

    let principal = quantity.abs() * Decimal::from(REPO_FACE_VALUE);
    let delta = cash_amount - principal;

    if is_buy {
        append_buy_fee_or_rounding(tx, delta, cash_currency, fee_account, rounding_account)
    } else {
        append_repo_interest_or_loss(tx, delta, cash_currency, interest_account, fee_account)
    }
}
