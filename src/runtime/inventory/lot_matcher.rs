//! 库存 lot 匹配器。
//!
//! 本模块负责从库存 lot 列表中按 FIFO 规则消费数量，并在需要时按目标成本过滤可消费 lot。
//! 卖出分录拆分会基于这里返回的匹配片段来生成。

use rust_decimal::Decimal;

use crate::model::account::cost::Cost;

use super::InventoryLot;

/// 单次 lot 消费返回的匹配片段。
#[derive(Debug, Clone)]
pub(super) struct MatchedLot {
    /// 本次从某个 lot 实际匹配到的数量，始终为正值。
    pub(super) quantity: Decimal,
    /// 匹配片段对应的成本信息。
    pub(super) cost: Cost,
}

/// 按 FIFO 从 lot 列表中消费指定数量，并返回匹配片段与未匹配数量。
///
/// 参数约束与语义：
/// - `remaining` 应传入正数，表示待消费的绝对数量；
/// - `target_cost` 为 `Some` 时，仅消费满足成本约束的 lot；
/// - 返回的第二个值若非零，表示库存不足或成本过滤后无法完全匹配。
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
        // 已耗尽 lot 可直接跳过，避免产生 0 数量匹配。
        if lot.remaining.is_zero() {
            continue;
        }
        // 在指定目标成本时，只允许消费完全匹配的 lot。
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

    // 清理已完全消费的 lot，保持库存结构紧凑。
    lots.retain(|lot| !lot.remaining.is_zero());
    (matched_lots, remaining)
}

/// 判断库存 lot 是否满足目标成本约束。
///
/// 匹配规则：
/// - `number`、`currency`、`label` 必须严格一致；
/// - 若目标成本未指定日期，则视为“任意日期可匹配”；
/// - 若目标成本指定了日期，则必须与 lot 日期一致。
pub(super) fn cost_matches(lot_cost: &Cost, target_cost: &Cost) -> bool {
    let same_number = lot_cost.number == target_cost.number;
    let same_currency = lot_cost.currency == target_cost.currency;
    let same_label = lot_cost.label == target_cost.label;
    let same_date = target_cost.date.is_none() || lot_cost.date == target_cost.date;
    same_number && same_currency && same_label && same_date
}
