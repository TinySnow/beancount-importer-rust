//! 预算分配旁线工具（独立于 importer 主流程）。
//!
//! 该工具读取 Beancount 账本与预算配置，按月汇总“预算 vs 实际”：
//! - 交易级 `budget` metadata 优先；
//! - 未设置 metadata 时，按账户前缀映射到默认预算桶；
//! - 最终输出每个预算桶的 planned/actual/remain。
//!
//! 说明：
//! - 该文件作为 `src/bin/*` 独立二进制存在，不会影响 importer 主流程。
//! - 只统计 `Expenses:*` 且金额为正的过账，避免与资产/负债分录混淆。

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, Result, anyhow, bail};
use chrono::{Datelike, NaiveDate};
use clap::Parser;
use once_cell::sync::Lazy;
use regex::Regex;
use rust_decimal::Decimal;
use serde::Deserialize;

/// Beancount 交易头：`YYYY-MM-DD * ...` 或 `YYYY-MM-DD ! ...`
static TX_HEADER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?P<date>\d{4}-\d{2}-\d{2})\s+[*!](?:\s|$)").expect("valid tx header regex")
});

/// 交易 metadata 行：`  key: value`
///
/// 注意：这里要求 `key` 后必须是 `:<空格>`，避免把
/// `Expenses:Food  10 CNY` 误判为 metadata。
static TX_METADATA_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s{2}(?P<key>[A-Za-z_][A-Za-z0-9_]*)\s*:\s+(?P<value>.+?)\s*$")
        .expect("valid tx metadata regex")
});

/// 过账行（可选金额）：
/// `  Expenses:Food  10 CNY`
/// `  Income:Misc`
static POSTING_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^\s{2}(?:[*!]\s+)?(?P<account>\S+)(?:\s{2,}(?P<number>[+-]?\d+(?:\.\d+)?)(?:\s+(?P<currency>[A-Za-z0-9_.-]+))?)?",
    )
    .expect("valid posting regex")
});

#[derive(Parser, Debug)]
#[command(name = "budget-report")]
#[command(about = "Budget sidecar report for Beancount ledgers")]
struct Cli {
    /// 账本文件路径（可重复传入多个）
    #[arg(long = "ledger", short = 'l', required = true)]
    ledgers: Vec<PathBuf>,

    /// 统计月份（YYYY-MM）
    #[arg(long, short = 'm')]
    month: String,

    /// 预算配置文件
    #[arg(long, default_value = "budget/budgets.yaml")]
    budgets: PathBuf,

    /// 默认映射配置文件
    #[arg(long, default_value = "budget/mappings.yaml")]
    mappings: PathBuf,

    /// 统计币种（默认 CNY）
    #[arg(long, default_value = "CNY")]
    currency: String,

    /// 严格模式：若存在未分配或未知预算桶则返回非零退出码
    #[arg(long)]
    strict: bool,
}

#[derive(Debug, Deserialize)]
struct BudgetMappings {
    /// account_prefix -> budget_bucket
    #[serde(default)]
    defaults: BTreeMap<String, String>,
}

#[derive(Debug)]
struct LedgerTransaction {
    date: NaiveDate,
    metadata: HashMap<String, String>,
    postings: Vec<LedgerPosting>,
}

#[derive(Debug)]
struct LedgerPosting {
    account: String,
    amount: Option<Decimal>,
    currency: Option<String>,
}

#[derive(Debug, Default)]
struct BucketSummary {
    planned: Decimal,
    actual: Decimal,
}

#[derive(Debug, Default)]
struct WarningStats {
    unassigned_amount: Decimal,
    unassigned_count: usize,
    unknown_bucket_amount: Decimal,
    unknown_bucket_names: BTreeSet<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    validate_month(&cli.month)?;

    let budgets = load_monthly_budgets(&cli.budgets)
        .with_context(|| format!("Failed to load budgets: {}", cli.budgets.display()))?;
    let mappings = load_mappings(&cli.mappings)
        .with_context(|| format!("Failed to load mappings: {}", cli.mappings.display()))?;

    let month_budgets = budgets.get(&cli.month).cloned().ok_or_else(|| {
        anyhow!(
            "Month '{}' not found in {}",
            cli.month,
            cli.budgets.display()
        )
    })?;

    let mut summaries: BTreeMap<String, BucketSummary> = month_budgets
        .iter()
        .map(|(bucket, planned)| {
            (
                bucket.clone(),
                BucketSummary {
                    planned: *planned,
                    actual: Decimal::ZERO,
                },
            )
        })
        .collect();

    let target_currency = cli.currency.to_ascii_uppercase();
    let mut warnings = WarningStats::default();

    for ledger in &cli.ledgers {
        let transactions = parse_ledger_file(ledger)
            .with_context(|| format!("Failed to parse ledger: {}", ledger.display()))?;

        for tx in transactions {
            if format!("{:04}-{:02}", tx.date.year(), tx.date.month()) != cli.month {
                continue;
            }

            let bucket_override = tx
                .metadata
                .get("budget")
                .cloned()
                // 兼容可能的误拼写，避免历史数据直接丢分类。
                .or_else(|| tx.metadata.get("budge").cloned())
                .and_then(|value| {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                });

            for posting in tx.postings {
                if !posting.account.starts_with("Expenses:") {
                    continue;
                }

                let Some(amount) = posting.amount else {
                    continue;
                };
                if !amount.is_sign_positive() {
                    continue;
                }

                let posting_currency = posting
                    .currency
                    .unwrap_or_else(|| target_currency.clone())
                    .to_ascii_uppercase();
                if posting_currency != target_currency {
                    continue;
                }

                let bucket = bucket_override
                    .clone()
                    .or_else(|| resolve_bucket_by_account(&mappings, &posting.account));

                match bucket {
                    Some(bucket_name) => {
                        let entry = summaries.entry(bucket_name.clone()).or_default();
                        entry.actual += amount;

                        if !month_budgets.contains_key(&bucket_name) {
                            warnings.unknown_bucket_amount += amount;
                            warnings.unknown_bucket_names.insert(bucket_name);
                        }
                    }
                    None => {
                        warnings.unassigned_amount += amount;
                        warnings.unassigned_count += 1;
                    }
                }
            }
        }
    }

    print_report(&cli.month, &target_currency, &summaries, &warnings);

    if cli.strict {
        if !warnings.unassigned_amount.is_zero() {
            bail!(
                "Strict mode failed: unassigned expenses = {} {}",
                warnings.unassigned_amount,
                target_currency
            );
        }
        if !warnings.unknown_bucket_amount.is_zero() {
            bail!(
                "Strict mode failed: unknown budget buckets amount = {} {}",
                warnings.unknown_bucket_amount,
                target_currency
            );
        }
    }

    Ok(())
}

fn validate_month(raw: &str) -> Result<()> {
    let mut parts = raw.split('-');
    let year = parts
        .next()
        .ok_or_else(|| anyhow!("Invalid month '{}'", raw))?;
    let month = parts
        .next()
        .ok_or_else(|| anyhow!("Invalid month '{}'", raw))?;
    if parts.next().is_some() {
        bail!("Invalid month '{}', expected YYYY-MM", raw);
    }

    let year: i32 = year
        .parse()
        .with_context(|| format!("Invalid year in month '{}'", raw))?;
    let month: u32 = month
        .parse()
        .with_context(|| format!("Invalid month number in '{}'", raw))?;
    if NaiveDate::from_ymd_opt(year, month, 1).is_none() {
        bail!("Invalid month '{}', expected YYYY-MM", raw);
    }
    Ok(())
}

fn load_monthly_budgets(path: &Path) -> Result<BTreeMap<String, BTreeMap<String, Decimal>>> {
    let content = fs::read_to_string(path)?;
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
    serde_yaml::from_str(content).context("Invalid budgets YAML")
}

fn load_mappings(path: &Path) -> Result<BudgetMappings> {
    let content = fs::read_to_string(path)?;
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
    serde_yaml::from_str(content).context("Invalid mappings YAML")
}

fn resolve_bucket_by_account(mappings: &BudgetMappings, account: &str) -> Option<String> {
    // 最长前缀优先，避免 `Expenses:Consume` 吃掉更具体的 `Expenses:Consume:电子`。
    mappings
        .defaults
        .iter()
        .filter(|(prefix, _)| account.starts_with(prefix.as_str()))
        .max_by_key(|(prefix, _)| prefix.len())
        .map(|(_, bucket)| bucket.clone())
}

fn parse_ledger_file(path: &Path) -> Result<Vec<LedgerTransaction>> {
    let content = fs::read_to_string(path)?;
    parse_ledger_content(&content)
}

fn parse_ledger_content(content: &str) -> Result<Vec<LedgerTransaction>> {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);

    #[derive(Debug)]
    struct Builder {
        date: NaiveDate,
        metadata: HashMap<String, String>,
        postings: Vec<LedgerPosting>,
    }

    impl Builder {
        fn finish(self) -> LedgerTransaction {
            LedgerTransaction {
                date: self.date,
                metadata: self.metadata,
                postings: self.postings,
            }
        }
    }

    let mut transactions = Vec::new();
    let mut current: Option<Builder> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim_end();

        if line.trim().is_empty() {
            if let Some(done) = current.take() {
                transactions.push(done.finish());
            }
            continue;
        }

        if let Some(header) = TX_HEADER_RE.captures(line) {
            if let Some(done) = current.take() {
                transactions.push(done.finish());
            }

            let date = NaiveDate::parse_from_str(&header["date"], "%Y-%m-%d")
                .with_context(|| format!("Invalid transaction date '{}'", &header["date"]))?;
            current = Some(Builder {
                date,
                metadata: HashMap::new(),
                postings: Vec::new(),
            });
            continue;
        }

        let Some(builder) = current.as_mut() else {
            continue;
        };

        if let Some(meta) = TX_METADATA_RE.captures(line) {
            let key = meta["key"].to_string();
            let value = parse_metadata_value(&meta["value"]);
            builder.metadata.insert(key, value);
            continue;
        }

        if let Some(posting) = POSTING_RE.captures(line) {
            let account = posting["account"].to_string();
            let amount = posting
                .name("number")
                .and_then(|raw| Decimal::from_str(raw.as_str()).ok());
            let currency = posting
                .name("currency")
                .map(|raw| raw.as_str().trim().to_string())
                .filter(|value| !value.is_empty());

            builder.postings.push(LedgerPosting {
                account,
                amount,
                currency,
            });
            continue;
        }
    }

    if let Some(done) = current.take() {
        transactions.push(done.finish());
    }

    Ok(transactions)
}

fn parse_metadata_value(raw: &str) -> String {
    let trimmed = raw.trim();

    if let Some(unquoted) = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        return unquoted
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
            .trim()
            .to_string();
    }

    if let Some(unquoted) = trimmed
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
    {
        return unquoted.trim().to_string();
    }

    trimmed.to_string()
}

fn print_report(
    month: &str,
    currency: &str,
    summaries: &BTreeMap<String, BucketSummary>,
    warnings: &WarningStats,
) {
    println!("Budget Report ({}) [{}]", month, currency);
    println!(
        "{:<24} {:>14} {:>14} {:>14} {:>10}",
        "Bucket", "Planned", "Actual", "Remain", "Status"
    );
    println!("{}", "-".repeat(82));

    let mut total_planned = Decimal::ZERO;
    let mut total_actual = Decimal::ZERO;

    for (bucket, summary) in summaries {
        let remain = summary.planned - summary.actual;
        let status = if remain.is_sign_negative() {
            "OVER"
        } else {
            "OK"
        };

        total_planned += summary.planned;
        total_actual += summary.actual;

        println!(
            "{:<24} {:>14} {:>14} {:>14} {:>10}",
            bucket, summary.planned, summary.actual, remain, status
        );
    }

    println!("{}", "-".repeat(82));
    let total_remain = total_planned - total_actual;
    let total_status = if total_remain.is_sign_negative() {
        "OVER"
    } else {
        "OK"
    };
    println!(
        "{:<24} {:>14} {:>14} {:>14} {:>10}",
        "TOTAL", total_planned, total_actual, total_remain, total_status
    );

    if !warnings.unassigned_amount.is_zero() {
        println!(
            "WARNING: unassigned expenses = {} {} ({} postings)",
            warnings.unassigned_amount, currency, warnings.unassigned_count
        );
    }

    if !warnings.unknown_bucket_amount.is_zero() {
        let names = warnings
            .unknown_bucket_names
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "WARNING: unknown buckets amount = {} {} (buckets: {})",
            warnings.unknown_bucket_amount, currency, names
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BudgetMappings, parse_ledger_content, parse_metadata_value, resolve_bucket_by_account,
    };
    use rust_decimal_macros::dec;
    use std::collections::BTreeMap;

    #[test]
    fn parses_budget_metadata_and_expense_posting() {
        let ledger = concat!(
            "2026-05-10 * \"iPad\"\n",
            "  budget: \"electronics\"\n",
            "  Expenses:Consume:电子  4999 CNY\n",
            "  Liabilities:CreditCard  -4999 CNY\n",
        );

        let txs = parse_ledger_content(ledger).expect("ledger parse should succeed");
        assert_eq!(txs.len(), 1);
        assert_eq!(
            txs[0].metadata.get("budget").map(String::as_str),
            Some("electronics")
        );
        assert_eq!(txs[0].postings.len(), 2);
        assert_eq!(txs[0].postings[0].account, "Expenses:Consume:电子");
        assert_eq!(txs[0].postings[0].amount, Some(dec!(4999)));
    }

    #[test]
    fn metadata_parser_unquotes_values() {
        assert_eq!(parse_metadata_value("\"electronics\""), "electronics");
        assert_eq!(parse_metadata_value(" 'travel' "), "travel");
        assert_eq!(parse_metadata_value("unquoted"), "unquoted");
    }

    #[test]
    fn longest_prefix_mapping_wins() {
        let mappings = BudgetMappings {
            defaults: BTreeMap::from([
                ("Expenses:Consume".to_string(), "consume".to_string()),
                (
                    "Expenses:Consume:电子".to_string(),
                    "electronics".to_string(),
                ),
            ]),
        };

        let bucket = resolve_bucket_by_account(&mappings, "Expenses:Consume:电子:配件")
            .expect("should match");
        assert_eq!(bucket, "electronics");
    }
}
