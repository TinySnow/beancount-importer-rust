use std::{fs, path::Path};

use anyhow::{Context, Result};
use log::{debug, warn};

use super::super::currency::is_fiat_currency;
use super::lot_matcher::consume_lots;
use super::seed_parser::{parse_seed_posting_line, parse_seed_transaction_date};
use super::{InventoryLot, InventoryState};

/// 从给定 seed 文件列表加载库存状态。
///
/// 文件解析失败不会中断主流程，只记录 warning。
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
fn ingest_seed_inventory_file(path: &Path, inventory: &mut InventoryState) -> Result<()> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read inventory seed file: {}", path.display()))?;

    let mut current_date: Option<chrono::NaiveDate> = None;

    for line in content.lines() {
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
            let Some(mut cost) = parsed.cost else {
                continue;
            };
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

        // seed 卖出用于回放库存变化，只消费匹配 lot，不额外生成残余记录。
        let _ = consume_lots(lots, parsed.quantity.abs(), Some(&target_cost));
    }

    Ok(())
}
