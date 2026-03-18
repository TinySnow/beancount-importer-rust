//! 模块说明：证券库存 lot 匹配、种子加载与成本补全能力。
//!
//! 文件路径：src/runtime/inventory/mod.rs。
//! 该文件主要承担子模块声明与导出职责。
//! 关键符号：无显式公开符号，主要通过内部实现或模块组织发挥作用。

use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::model::{account::cost::Cost, transaction::Transaction};

mod lot_apply;
mod lot_matcher;
mod seed_loader;
mod seed_parser;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub(super) struct InventoryLot {
    /// 当前 lot 剩余可用数量。
    pub(super) remaining: Decimal,
    /// 当前 lot 的成本信息。
    pub(super) cost: Cost,
}

/// 运行期库存状态：按 `(账户, 商品)` 维度保存 lot 列表。
#[derive(Debug, Default)]
pub(crate) struct InventoryState {
    pub(super) lots: HashMap<(String, String), Vec<InventoryLot>>,
}

/// 测试辅助：不加载 seed 文件，仅使用当前批次交易推导 lot。
#[cfg(test)]
pub(crate) fn resolve_inferred_cost_postings(transactions: &mut [Transaction]) {
    let mut inventory = InventoryState::default();
    resolve_inferred_cost_postings_with_inventory(transactions, &mut inventory);
}

/// 使用给定库存状态解析交易中的推断成本分录（`{}`）。
pub(crate) fn resolve_inferred_cost_postings_with_inventory(
    transactions: &mut [Transaction],
    inventory: &mut InventoryState,
) {
    lot_apply::resolve_inferred_cost_postings_with_inventory(transactions, inventory);
}

/// 从 seed 文件批量加载库存状态。
pub(crate) fn load_seed_inventory_from_files(paths: &[String]) -> InventoryState {
    seed_loader::load_seed_inventory_from_files(paths)
}
