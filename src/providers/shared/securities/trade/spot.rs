//! 普通证券买卖分录构建。
//!
//! 负责现货买卖场景下的持仓、现金腿和费用/盈亏补充分录。

use rust_decimal::Decimal;

use crate::model::{
    account::{amount::Amount, cost::Cost, posting::Posting, price::Price},
    transaction::Transaction,
};

use super::super::posting::{append_buy_fee_or_rounding, append_fee_delta};

/// 普通证券买卖分录构建输入。
pub(super) struct SpotPostingInput<'a> {
    /// 待追加分录的交易对象。
    pub(super) tx: Transaction,
    /// 证券持仓账户。
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
    /// 原始数量绝对值（用于差额计算）。
    pub(super) quantity: Decimal,
    /// 原始现金金额绝对值（用于差额计算）。
    pub(super) cash_amount: Decimal,
    /// 是否买入方向。
    pub(super) is_buy: bool,
    /// 生效单价（原始单价或由金额/数量反推）。
    pub(super) effective_price: Decimal,
    /// 手续费账户。
    pub(super) fee_account: &'a str,
    /// 舍入差异账户。
    pub(super) rounding_account: &'a str,
    /// 已实现盈亏账户。
    pub(super) pnl_account: &'a str,
}

/// 应用普通证券买卖分录。
///
/// 规则：
/// - 买入：持仓使用 `{成本}` 记法。
/// - 卖出：持仓使用 `{}` + `@ 市价` 触发成本匹配，并加 PnL 平衡分录。
pub(super) fn apply_spot_postings(input: SpotPostingInput<'_>) -> Transaction {
    let SpotPostingInput {
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
        effective_price,
        fee_account,
        rounding_account,
        pnl_account,
    } = input;

    let mut holdings_posting = Posting::new(holdings_account)
        .with_amount(Amount::new(signed_quantity, commodity_symbol.to_string()));

    if is_buy {
        holdings_posting =
            holdings_posting.with_cost(Cost::new(effective_price, cash_currency.to_string()));
    } else {
        holdings_posting = holdings_posting
            .with_inferred_cost()
            .with_price(Price::new(effective_price, cash_currency.to_string()));
    }

    tx = tx.with_posting(holdings_posting);
    tx = tx.with_posting(
        Posting::new(cash_account).with_amount(Amount::new(signed_cash, cash_currency.to_string())),
    );

    // 差额 = 实际现金 与 理论成交额 的差，用于补记费用或舍入误差。
    let fee_delta = if is_buy {
        cash_amount - quantity.abs() * effective_price
    } else {
        quantity.abs() * effective_price - cash_amount
    };

    if is_buy {
        append_buy_fee_or_rounding(tx, fee_delta, cash_currency, fee_account, rounding_account)
    } else {
        let tx = append_fee_delta(tx, fee_delta, cash_currency, fee_account);
        tx.with_posting(Posting::new(pnl_account))
    }
}
