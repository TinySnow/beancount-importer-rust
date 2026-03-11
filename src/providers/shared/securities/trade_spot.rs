use rust_decimal::Decimal;

use crate::model::{
    account::{amount::Amount, cost::Cost, posting::Posting, price::Price},
    transaction::Transaction,
};

use super::posting::{append_buy_fee_or_rounding, append_fee_delta};

/// 普通证券买卖分录构建输入。
pub(super) struct SpotPostingInput<'a> {
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
    pub(super) effective_price: Decimal,
    pub(super) fee_account: &'a str,
    pub(super) rounding_account: &'a str,
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
