//! seed 库存加载器。
//!
//! 本模块负责从外部 seed 文件回放历史持仓变化，构建初始库存状态，
//! 以支持当前批次交易进行跨期 lot 匹配。

use std::{fs, path::Path};

use crate::utils::currency_kind::is_fiat_currency;
use anyhow::{Context, Result};
use log::{debug, warn};

use super::lot_matcher::consume_lots;
use super::seed_parser::{parse_seed_posting_line, parse_seed_transaction_date};
use super::{InventoryLot, InventoryState};

/// 从给定 seed 文件列表加载库存状态。
///
/// 设计目标是“尽力加载”：
/// - 某个文件读取或解析失败仅记录 warning；
/// - 其余文件仍继续处理；
/// - 最终返回已成功回放得到的库存状态。
pub(super) fn load_seed_inventory_from_files(paths: &[String]) -> InventoryState {
    if paths.is_empty() {
        return InventoryState::default();
    }

    let mut inventory = InventoryState::default();
    for path in paths {
        let seed_path = Path::new(path);
        match ingest_seed_inventory_file(seed_path, &mut inventory) {
            Ok(()) => debug!("Loaded inventory seed file: {}", seed_path.display()),
            Err(error) => warn!(
                "Failed to load inventory seed file '{}': {}",
                seed_path.display(),
                error
            ),
        }
    }

    inventory
}

/// 解析单个 seed 文件，并把分录变化应用到库存状态。
///
/// 支持的回放行为：
/// - 买入（正数量）会新增 lot；
/// - 卖出（负数量）会按成本约束消费 lot；
/// - 法币分录与无效分录会被忽略。
fn ingest_seed_inventory_file(path: &Path, inventory: &mut InventoryState) -> Result<()> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read inventory seed file: {}", path.display()))?;

    let mut current_date: Option<chrono::NaiveDate> = None;

    for line in content.lines() {
        // 交易头用于刷新 fallback 日期，供后续成本缺日期时回填。
        if let Some(tx_date) = parse_seed_transaction_date(line) {
            current_date = Some(tx_date);
            continue;
        }

        let Some(parsed) = parse_seed_posting_line(line, current_date) else {
            continue;
        };

        if is_fiat_currency(&parsed.commodity) {
            continue;
        }

        let key = (parsed.account, parsed.commodity);
        let lots = inventory.lots.entry(key).or_default();

        if parsed.quantity.is_sign_positive() {
            // seed 买入必须携带成本，否则无法构建可消费 lot。
            let Some(mut cost) = parsed.cost else {
                continue;
            };
            // 若成本未显式给出日期，则使用当前交易日期作为 lot 日期。
            if cost.date.is_none() {
                cost.date = current_date;
            }
            lots.push(InventoryLot {
                remaining: parsed.quantity,
                cost,
            });
            continue;
        }

        if !parsed.quantity.is_sign_negative() {
            continue;
        }

        let Some(target_cost) = parsed.cost else {
            continue;
        };

        // seed 卖出仅用于回放库存变化：消费匹配 lot，不生成残余分录。
        let _ = consume_lots(lots, parsed.quantity.abs(), Some(&target_cost));
    }

    Ok(())
}
