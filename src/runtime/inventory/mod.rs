//! 运行期库存模块。
//!
//! 该模块用于在导入阶段维护证券/商品库存（lot）状态，并为卖出分录补全可确定的成本信息。
//!
//! 核心规则：
//! - 按 `(账户, 商品)` 维度维护库存 lot；
//! - 买入分录会注册为可消费 lot；
//! - 卖出分录按 FIFO 消费库存，并在可匹配时拆分为带明确成本的分录；
//! - 可从 seed 文件预加载跨期库存状态（[`seed`] 子模块）。
//!
//! # 模块结构
//! - [`lot_matcher`]：核心 FIFO lot 匹配算法；
//! - [`lot_apply`]：将匹配结果应用到交易分录；
//! - [`seed`]：从外部 Beancount 文件回放历史库存。

use std::collections::HashMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::model::{account::cost::Cost, transaction::Transaction};

mod lot_apply;
mod lot_matcher;
pub(crate) mod seed;

#[cfg(test)]
mod tests;

/// 库存中的单个 lot。
///
/// 一个 lot 表示某次买入后尚未被卖出消费的剩余数量及其成本信息。
#[derive(Debug, Clone)]
pub(super) struct InventoryLot {
    /// 当前 lot 剩余可用数量，始终为正值。
    pub(super) remaining: Decimal,
    /// 当前 lot 的成本信息，用于后续卖出分录补全成本。
    pub(super) cost: Cost,
}

/// 运行期库存状态。
///
/// `lots` 使用 `(账户, 商品)` 作为键，值为该维度下按时间顺序维护的 lot 列表。
#[derive(Debug, Default)]
pub(crate) struct InventoryState {
    /// 按 `(账户, 商品)` 分组的库存 lot 队列。
    pub(super) lots: HashMap<(String, String), Vec<InventoryLot>>,
}

/// 测试辅助入口：不加载 seed 文件，仅使用当前交易切片推导库存并补全卖出成本。
#[cfg(test)]
pub(crate) fn resolve_inferred_cost_postings(transactions: &mut [Transaction]) {
    let mut inventory = InventoryState::default();
    resolve_inferred_cost_postings_with_inventory(transactions, &mut inventory);
}

/// 使用给定库存状态补全交易中的卖出成本。
///
/// 该函数会处理两类待补全分录：
/// - 使用推断成本 `{}` 的卖出分录；
/// - 显式成本存在但未带日期、需要按 FIFO lot 回填日期的卖出分录。
pub(crate) fn resolve_inferred_cost_postings_with_inventory(
    transactions: &mut [Transaction],
    inventory: &mut InventoryState,
) {
    lot_apply::resolve_inferred_cost_postings_with_inventory(transactions, inventory);
}

/// 从 seed 文件批量加载库存状态。
///
/// `cutoff` 为可选截止日期：日期达到或超过该截止点的 seed 交易会被跳过，
/// 用于排除当前批次自身的历史记录（自引用）并保持 FIFO 时间序正确。
///
/// 解析失败的文件会被跳过并记录日志，不会中断主流程。
pub(crate) fn load_seed_inventory_from_files(
    paths: &[String],
    cutoff: Option<NaiveDate>,
) -> InventoryState {
    seed::load_seed_inventory_from_files(paths, cutoff)
}
