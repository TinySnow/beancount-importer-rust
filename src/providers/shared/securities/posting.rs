//! 证券交易中的差额分录附加工具。
//!
//! 该模块处理“理论金额与实际金额差额”的落账策略，例如手续费、
//! 舍入差异、逆回购利息/损失等。

use rust_decimal::Decimal;

use crate::model::{
    account::{amount::Amount, posting::Posting},
    transaction::Transaction,
};

/// 买入场景差额处理。
///
/// - 正差额：记入手续费账户。
/// - 负差额：记入舍入差异账户。
///
/// `delta` 的计算由调用方完成，这里只负责按符号分流到账户。
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
///
/// `delta` 可能为正或负，按原符号写入，便于保留 Provider 给出的净额语义。
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
///
/// 这里把利息收入记为负值，是为了与 Beancount 中收入账户“贷方增加”的
/// 记账符号保持一致。
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
