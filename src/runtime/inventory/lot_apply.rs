//! lot 应用与分录改写逻辑。
//!
//! 本模块负责把库存规则应用到交易分录：
//! - 买入分录登记为可消费 lot；
//! - 卖出分录在可匹配时按 lot 拆分为多条带明确成本的分录；
//! - 匹配不足时保留残余分录，维持原始语义。

use rust_decimal::Decimal;

use crate::model::{
    account::{cost::Cost, posting::Posting},
    transaction::Transaction,
};

use super::super::currency::is_fiat_currency;
use super::lot_matcher::consume_lots;
use super::{InventoryLot, InventoryState};

/// 使用给定库存状态改写交易中的卖出分录成本。
///
/// 处理策略：
/// - 对买入分录：若有成本信息，则写入库存 lot；
/// - 对卖出分录：若为推断成本 `{}` 或显式成本缺日期，则尝试按 FIFO lot 拆分；
/// - 若库存不足或无可匹配 lot，则保留原始分录供后续流程处理。
pub(super) fn resolve_inferred_cost_postings_with_inventory(
    transactions: &mut [Transaction],
    inventory: &mut InventoryState,
) {
    for tx in transactions {
        let mut rewritten = Vec::with_capacity(tx.postings.len());

        for posting in tx.postings.drain(..) {
            // 无金额分录无法参与库存增减，直接透传。
            let Some(amount) = posting.amount.as_ref() else {
                rewritten.push(posting);
                continue;
            };

            let commodity = amount.currency.clone();
            // 法币分录不进入证券 lot 库存。
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

            // 仅在“确实需要补全成本语义”时才做拆分，避免改写无关分录。
            if !should_split_sell_posting(&posting, amount_number) {
                rewritten.push(posting);
                continue;
            }

            // 推断成本 `{}` 不带目标成本约束；显式成本时按目标成本过滤 lot。
            let target_cost = if posting.inferred_cost {
                None
            } else {
                posting.cost.as_ref()
            };

            let lots = inventory.lots.entry(key).or_default();
            let (matched_lots, remaining) = consume_lots(lots, amount_number.abs(), target_cost);

            if matched_lots.is_empty() {
                // 无法匹配到任何 lot，保留原分录，避免在信息不足时引入错误成本。
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

            // 若只匹配了部分数量，残余部分继续保留原成本语义（推断或显式）。
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
///
/// 当分录提供了成本信息时：
/// - 使用买入数量作为 lot 可用数量；
/// - 若成本未显式提供日期，则回填交易日期作为 lot 日期。
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
///
/// 返回 `true` 的条件：
/// - 金额为负（卖出）且 `inferred_cost = true`；
/// - 或金额为负且显式成本存在但缺少日期（需要从 lot 回填日期）。
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
///
/// 拆分后会把数量改为负值（卖出方向），并将 `inferred_cost` 设为 `false`。
fn build_sell_split_posting(template: &Posting, quantity: Decimal, cost: Cost) -> Option<Posting> {
    let mut posting = template.clone();
    let amount = posting.amount.as_mut()?;

    amount.number = -quantity;
    posting.cost = Some(cost);
    posting.inferred_cost = false;

    Some(posting)
}

/// 构造一条残余卖出分录（仍保留原始成本语义）。
///
/// 残余分录用于表示“库存不足导致未被 lot 完全覆盖”的卖出部分。
fn build_sell_residual_posting(template: &Posting, remaining: Decimal) -> Option<Posting> {
    let mut posting = template.clone();
    let amount = posting.amount.as_mut()?;
    amount.number = -remaining;
    Some(posting)
}
