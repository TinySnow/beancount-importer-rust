//! seed 库存加载器。
//!
//! 本模块负责从外部 seed 文件回放历史持仓变化，构建初始库存状态，
//! 以支持当前批次交易进行跨期 lot 匹配。

use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::NaiveDate;
use log::{debug, warn};

use crate::{
    error::{ImporterError, ImporterResult},
    utils::currency_kind::is_fiat_currency,
};

use super::super::lot_matcher::consume_lots;
use super::parser::{parse_seed_posting_line, parse_seed_transaction_date};
use super::super::{InventoryLot, InventoryState};

/// 从给定 seed 文件列表加载库存状态。
///
/// 设计目标是“尽力加载”：
/// - 某个文件读取或解析失败仅记录 warning；
/// - 其余文件仍继续处理；
/// - 最终返回已成功回放得到的库存状态。
///
/// `cutoff` 为可选截止日期：日期达到或超过该截止点的 seed 交易会被跳过，
/// 用于排除当前批次自身的历史记录（自引用）。
pub(crate) fn load_seed_inventory_from_files(
    paths: &[String],
    cutoff: Option<NaiveDate>,
) -> InventoryState {
    if paths.is_empty() {
        return InventoryState::default();
    }

    let mut inventory = InventoryState::default();
    for path in paths {
        let seed_path = Path::new(path);
        // 支持目录：自动扫描其中所有 .bean / .beancount 文件
        if seed_path.is_dir() {
            collect_bean_files(seed_path, &mut inventory, cutoff);
        } else {
            ingest_one(seed_path, &mut inventory, cutoff);
        }
    }

    inventory
}

/// 递归扫描目录中的 .bean / .beancount 文件，按路径排序后回放库存。
///
/// `transactions/YYYY/MM/*.bean` 的目录结构使路径排序天然等价于时间序，
/// 避免 `read_dir` 无序导致跨月 lot 被错误地先消费。
fn collect_bean_files(dir: &Path, inventory: &mut InventoryState, cutoff: Option<NaiveDate>) {
    let mut files = Vec::new();
    collect_bean_paths(dir, &mut files);
    files.sort();
    for file in files {
        ingest_one(&file, inventory, cutoff);
    }
}

/// 递归收集目录中所有 .bean / .beancount 文件路径。
///
/// 跳过名为 `tmp` 的目录：它是导入器的输出暂存目录（batch 模式写入
/// `src/transactions/tmp/`），其内容只是最终月账单的副本，作为 seed 回放会
/// 造成 lot 重复注册，进而导致卖出匹配到错误（重复）的 lot。
fn collect_bean_paths(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if is_staging_dir(&path) {
                    continue;
                }
                collect_bean_paths(&path, files);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("bean") || ext.eq_ignore_ascii_case("beancount") {
                    files.push(path);
                }
            }
        }
    }
}

/// 判断目录是否为导入器的输出暂存目录（如 `tmp`）。
fn is_staging_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case("tmp"))
        .unwrap_or(false)
}

fn ingest_one(path: &Path, inventory: &mut InventoryState, cutoff: Option<NaiveDate>) {
    match ingest_seed_inventory_file(path, inventory, cutoff) {
        Ok(()) => debug!("Loaded inventory seed file: {}", path.display()),
        Err(error) => warn!(
            "Failed to load inventory seed file '{}': {}",
            path.display(),
            error
        ),
    }
}

/// 解析单个 seed 文件，并把分录变化应用到库存状态。
///
/// 支持的回放行为：
/// - 买入（正数量）会新增 lot；
/// - 卖出（负数量）会按成本约束消费 lot；
/// - 法币分录与无效分录会被忽略；
/// - 日期达到或超过 `cutoff` 的交易会被整体跳过。
fn ingest_seed_inventory_file(
    path: &Path,
    inventory: &mut InventoryState,
    cutoff: Option<NaiveDate>,
) -> ImporterResult<()> {
    let content = fs::read_to_string(path).map_err(|e| {
        ImporterError::Io(e).with_context(format!(
            "Failed to read inventory seed file: {}",
            path.display()
        ))
    })?;

    let mut current_date: Option<NaiveDate> = None;
    let mut skip_current = false;

    for line in content.lines() {
        // 交易头用于刷新 fallback 日期，并据此判断是否达到截止点。
        if let Some(tx_date) = parse_seed_transaction_date(line) {
            current_date = Some(tx_date);
            skip_current = cutoff.map(|c| tx_date >= c).unwrap_or(false);
            continue;
        }

        // 达到或超过截止点的交易视为当前批次内容，跳过以避免自引用。
        if skip_current {
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

        // seed 卖出：若无显式日期则按成本值匹配（忽略日期，因 seed 中 buy/sell 交易日期不同）
        let target = parsed.cost.as_ref().map(|c| {
            let mut c = c.clone();
            if c.date.is_some() {
                // 检查是否为 seed parser 的 fallback_date：若日期格式=交易头日期，则移除
                // （seed 文件中 cost 原本无日期，parser 回填了交易日期作为 fallback）
                c.date = None;
            }
            c
        });
        let target_ref = target.as_ref();
        let _ = consume_lots(lots, parsed.quantity.abs(), target_ref);
    }

    Ok(())
}
