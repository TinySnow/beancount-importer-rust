//! seed 文件解析器。
//!
//! 本模块将 seed 文本行解析为结构化结果，供库存加载器回放持仓变化。
//! 解析范围聚焦于：
//! - 交易头日期行；
//! - 过账行的账户、数量、商品；
//! - 可选的成本表达式 `{number currency[, date][, "label"]}`。

use std::str::FromStr;

use once_cell::sync::Lazy;
use regex::Regex;
use rust_decimal::Decimal;

use crate::model::account::cost::Cost;

/// seed 文件中过账行的结构化结果。
#[derive(Debug)]
pub(super) struct ParsedSeedPosting {
    /// 过账账户名，例如 `Assets:Invest:Broker:Securities`。
    pub(super) account: String,
    /// 过账数量，正数表示买入，负数表示卖出。
    pub(super) quantity: Decimal,
    /// 商品代码（或证券代码）。
    pub(super) commodity: String,
    /// 可选成本信息；无成本表达式时为 `None`。
    pub(super) cost: Option<Cost>,
}

/// 解析 seed 文件中的交易头日期。
///
/// 支持形如 `YYYY-MM-DD *` 或 `YYYY-MM-DD !` 的行，返回解析出的交易日期。
pub(super) fn parse_seed_transaction_date(line: &str) -> Option<chrono::NaiveDate> {
    static TX_DATE_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^(?P<date>\d{4}-\d{2}-\d{2})\s+[*!]").expect("valid tx date regex")
    });

    let caps = TX_DATE_RE.captures(line.trim())?;
    chrono::NaiveDate::parse_from_str(caps.name("date")?.as_str(), "%Y-%m-%d").ok()
}

/// 解析 seed 文件中的过账行，可选解析成本表达式。
///
/// `fallback_date` 用于在成本表达式未显式提供日期时回填成本日期。
pub(super) fn parse_seed_posting_line(
    line: &str,
    fallback_date: Option<chrono::NaiveDate>,
) -> Option<ParsedSeedPosting> {
    static POSTING_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r#"^\s{2}(?:[*!]\s+)?(?P<account>\S+)\s+(?P<number>[+-]?\d+(?:\.\d+)?)\s+(?P<commodity>[A-Za-z0-9_.-]+)(?:\s+\{(?P<cost>[^}]*)\})?"#,
        )
        .expect("valid posting regex")
    });

    let caps = POSTING_RE.captures(line)?;
    let account = caps.name("account")?.as_str().to_string();
    let quantity = Decimal::from_str(caps.name("number")?.as_str()).ok()?;
    let commodity = caps.name("commodity")?.as_str().to_string();
    let cost = caps
        .name("cost")
        .and_then(|raw| parse_seed_cost(raw.as_str(), fallback_date));

    Some(ParsedSeedPosting {
        account,
        quantity,
        commodity,
        cost,
    })
}

/// 解析成本表达式：`number currency[, date][, "label"]`。
///
/// 解析规则：
/// - `number` 与 `currency` 为必填；
/// - `date` 可选，缺失时使用 `fallback_date`；
/// - `label` 可选，格式为双引号字符串。
fn parse_seed_cost(raw: &str, fallback_date: Option<chrono::NaiveDate>) -> Option<Cost> {
    static COST_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r#"^\s*(?P<number>[+-]?\d+(?:\.\d+)?)\s+(?P<currency>[A-Za-z0-9_.-]+)(?:,\s*(?P<date>\d{4}-\d{2}-\d{2}))?(?:,\s*\"(?P<label>[^\"]*)\")?\s*$"#,
        )
        .expect("valid cost regex")
    });

    let caps = COST_RE.captures(raw.trim())?;
    let number = Decimal::from_str(caps.name("number")?.as_str()).ok()?;
    let currency = caps.name("currency")?.as_str().to_string();
    let mut cost = Cost::new(number, currency);

    // 显式日期优先于 fallback 日期。
    if let Some(date_match) = caps.name("date") {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_match.as_str(), "%Y-%m-%d") {
            cost.date = Some(date);
        }
    } else {
        cost.date = fallback_date;
    }

    if let Some(label_match) = caps.name("label") {
        cost.label = Some(label_match.as_str().to_string());
    }

    Some(cost)
}
