use rust_decimal::Decimal;

use crate::model::{
    account::{amount::Amount, posting::Posting},
    transaction::Transaction,
};

/// 买入场景差额处理。
///
/// - 正差额：记入手续费账户。
/// - 负差额：记入舍入差异账户。
pub(super) fn append_buy_fee_or_rounding(
    mut tx: Transaction,
    delta: Decimal,
    currency: &str,
    fee_account: &str,
    rounding_account: &str,
) -> Transaction {
    if delta.is_zero() {
        return tx;
    }

    if delta.is_sign_positive() {
        tx = tx.with_posting(
            Posting::new(fee_account).with_amount(Amount::new(delta, currency.to_string())),
        );
    } else {
        tx = tx.with_posting(
            Posting::new(rounding_account).with_amount(Amount::new(delta, currency.to_string())),
        );
    }

    tx
}

/// 卖出场景差额处理：统一记入手续费账户。
pub(super) fn append_fee_delta(
    mut tx: Transaction,
    delta: Decimal,
    currency: &str,
    fee_account: &str,
) -> Transaction {
    if delta.is_zero() {
        return tx;
    }

    tx = tx.with_posting(
        Posting::new(fee_account).with_amount(Amount::new(delta, currency.to_string())),
    );

    tx
}

/// 逆回购到期差额处理。
///
/// - 正差额：记为利息收入（负号记入 Income）。
/// - 负差额：记为费用损失。
pub(super) fn append_repo_interest_or_loss(
    mut tx: Transaction,
    delta: Decimal,
    currency: &str,
    income_account: &str,
    expense_account: &str,
) -> Transaction {
    if delta.is_zero() {
        return tx;
    }

    if delta.is_sign_positive() {
        tx = tx.with_posting(
            Posting::new(income_account).with_amount(Amount::new(-delta, currency.to_string())),
        );
    } else {
        tx = tx.with_posting(
            Posting::new(expense_account)
                .with_amount(Amount::new(delta.abs(), currency.to_string())),
        );
    }

    tx
}
