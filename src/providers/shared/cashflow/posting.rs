//! 模块说明：跨 Provider 的现金流分类与分录构建能力。
//!
//! 文件路径：src/providers/shared/cashflow/posting.rs。
//! 该文件围绕 'posting' 的职责提供实现。
//! 关键符号：无显式公开符号，主要通过内部实现或模块组织发挥作用。

use rust_decimal::Decimal;

use crate::model::{
    account::{amount::Amount, posting::Posting},
    transaction::Transaction,
};

/// 为支出交易追加双分录。
///
/// 借：费用账户
/// 贷：资产账户
pub(super) fn apply_expense_postings(
    mut tx: Transaction,
    expense_account: &str,
    asset_account: &str,
    amount: Decimal,
    currency: &str,
) -> Transaction {
    tx = tx.with_posting(
        Posting::new(expense_account).with_amount(Amount::new(amount.abs(), currency.to_string())),
    );
    tx = tx.with_posting(
        Posting::new(asset_account).with_amount(Amount::new(-amount.abs(), currency.to_string())),
    );

    tx
}

/// 为收入交易追加双分录。
///
/// 借：资产账户
/// 贷：收入账户
pub(super) fn apply_income_postings(
    mut tx: Transaction,
    asset_account: &str,
    income_account: &str,
    amount: Decimal,
    currency: &str,
) -> Transaction {
    tx = tx.with_posting(
        Posting::new(asset_account).with_amount(Amount::new(amount.abs(), currency.to_string())),
    );
    tx = tx.with_posting(
        Posting::new(income_account).with_amount(Amount::new(-amount.abs(), currency.to_string())),
    );

    tx
}
