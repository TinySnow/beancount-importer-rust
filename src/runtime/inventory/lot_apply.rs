//! 模块说明：证券库存 lot 匹配、种子加载与成本补全能力。
//!
//! 文件路径：src/runtime/inventory/lot_apply.rs。
//! 该文件围绕 'lot_apply' 的职责提供实现。
//! 关键符号：register_buy_lot、should_split_sell_posting、build_sell_split_posting、build_sell_residual_posting。

use rust_decimal::Decimal;

use crate::model::{
    account::{cost::Cost, posting::Posting},
    transaction::Transaction,
};

use super::super::currency::is_fiat_currency;
use super::lot_matcher::consume_lots;
use super::{InventoryLot, InventoryState};

/// 使用当前库存状态解析卖出分录中的推断成本（`{}`）或缺失日期成本。
pub(super) fn resolve_inferred_cost_postings_with_inventory(
    transactions: &mut [Transaction],
    inventory: &mut InventoryState,
) {
    for tx in transactions {
        let mut rewritten = Vec::with_capacity(tx.postings.len());

        for posting in tx.postings.drain(..) {
            let Some(amount) = posting.amount.as_ref() else {
                rewritten.push(posting);
                continue;
            };

            let commodity = amount.currency.clone();
            if is_fiat_currency(&commodity) {
                rewritten.push(posting);
                continue;
            }

            let amount_number = amount.number;
            let key = (posting.account.clone(), commodity);

            // 买入侧：把 lot 记入库存，供后续卖出匹配。
            if amount_number.is_sign_positive() {
                register_buy_lot(inventory, key, &posting, tx.date);
                rewritten.push(posting);
                continue;
            }

            if !should_split_sell_posting(&posting, amount_number) {
                rewritten.push(posting);
                continue;
            }

            let target_cost = if posting.inferred_cost {
                None
            } else {
                posting.cost.as_ref()
            };

            let lots = inventory.lots.entry(key).or_default();
            let (matched_lots, remaining) = consume_lots(lots, amount_number.abs(), target_cost);

            if matched_lots.is_empty() {
                // 无法匹配到任何 lot，保留原始分录交给下游处理。
                rewritten.push(posting);
                continue;
            }

            for matched_lot in matched_lots {
                if let Some(split) =
                    build_sell_split_posting(&posting, matched_lot.quantity, matched_lot.cost)
                {
                    rewritten.push(split);
                }
            }

            // 若只匹配了部分数量，残余部分继续保留原语义。
            if !remaining.is_zero()
                && let Some(residual) = build_sell_residual_posting(&posting, remaining)
            {
                rewritten.push(residual);
            }
        }

        tx.postings = rewritten;
    }
}

/// 记录买入分录对应的 lot。
fn register_buy_lot(
    inventory: &mut InventoryState,
    key: (String, String),
    posting: &Posting,
    tx_date: chrono::NaiveDate,
) {
    let Some(amount) = posting.amount.as_ref() else {
        return;
    };
    let Some(cost) = posting.cost.as_ref() else {
        return;
    };

    let mut lot_cost = cost.clone();
    if lot_cost.date.is_none() {
        lot_cost.date = Some(tx_date);
    }

    inventory.lots.entry(key).or_default().push(InventoryLot {
        remaining: amount.number,
        cost: lot_cost,
    });
}

/// 判断卖出分录是否需要进行 lot 拆分。
fn should_split_sell_posting(posting: &Posting, amount_number: Decimal) -> bool {
    if !amount_number.is_sign_negative() {
        return false;
    }

    if posting.inferred_cost {
        return true;
    }

    posting
        .cost
        .as_ref()
        .map(|cost| cost.date.is_none())
        .unwrap_or(false)
}

/// 构造一条带明确成本的拆分卖出分录。
fn build_sell_split_posting(template: &Posting, quantity: Decimal, cost: Cost) -> Option<Posting> {
    let mut posting = template.clone();
    let amount = posting.amount.as_mut()?;

    amount.number = -quantity;
    posting.cost = Some(cost);
    posting.inferred_cost = false;

    Some(posting)
}

/// 构造一条残余卖出分录（仍保留原始成本语义）。
fn build_sell_residual_posting(template: &Posting, remaining: Decimal) -> Option<Posting> {
    let mut posting = template.clone();
    let amount = posting.amount.as_mut()?;
    amount.number = -remaining;
    Some(posting)
}
