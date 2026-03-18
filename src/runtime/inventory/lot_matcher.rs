//! 模块说明：证券库存 lot 匹配、种子加载与成本补全能力。
//!
//! 文件路径：src/runtime/inventory/lot_matcher.rs。
//! 该文件围绕 'lot_matcher' 的职责提供实现。
//! 关键符号：无显式公开符号，主要通过内部实现或模块组织发挥作用。

use rust_decimal::Decimal;

use crate::model::account::cost::Cost;

use super::InventoryLot;

/// 一次 lot 匹配的结果片段。
#[derive(Debug, Clone)]
pub(super) struct MatchedLot {
    pub(super) quantity: Decimal,
    pub(super) cost: Cost,
}

/// 按 FIFO 从 lot 列表中消费指定数量，并返回匹配到的 lot 片段。
///
/// - `remaining` 输入必须为正数；
/// - `target_cost` 为 `Some` 时，仅消费满足成本约束的 lot；
/// - 返回值中的剩余数量若非零，表示库存不足或未匹配到目标成本。
pub(super) fn consume_lots(
    lots: &mut Vec<InventoryLot>,
    mut remaining: Decimal,
    target_cost: Option<&Cost>,
) -> (Vec<MatchedLot>, Decimal) {
    let mut matched_lots = Vec::new();

    for lot in lots.iter_mut() {
        if remaining.is_zero() {
            break;
        }
        if lot.remaining.is_zero() {
            continue;
        }
        if let Some(target_cost) = target_cost
            && !cost_matches(&lot.cost, target_cost)
        {
            continue;
        }

        let matched = if lot.remaining <= remaining {
            lot.remaining
        } else {
            remaining
        };

        if matched.is_zero() {
            continue;
        }

        lot.remaining -= matched;
        remaining -= matched;

        matched_lots.push(MatchedLot {
            quantity: matched,
            cost: lot.cost.clone(),
        });
    }

    lots.retain(|lot| !lot.remaining.is_zero());
    (matched_lots, remaining)
}

/// 判断库存 lot 是否满足目标成本约束。
///
/// 规则：number/currency/label 必须一致；若目标成本未带日期，则忽略日期匹配。
pub(super) fn cost_matches(lot_cost: &Cost, target_cost: &Cost) -> bool {
    let same_number = lot_cost.number == target_cost.number;
    let same_currency = lot_cost.currency == target_cost.currency;
    let same_label = lot_cost.label == target_cost.label;
    let same_date = target_cost.date.is_none() || lot_cost.date == target_cost.date;
    same_number && same_currency && same_label && same_date
}
