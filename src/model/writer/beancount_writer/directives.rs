//! `open` / `commodity` 指令写出逻辑。
//!
//! 本模块为 [`super::BeancountWriter`] 提供账户与商品声明相关能力：
//! - 解析 `open` / `commodity` 指令日期；
//! - 汇总账户法币与非法币持仓特征；
//! - 规范化 `booking_method`；
//! - 收集需要声明的商品代码。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::{
//!     account::{amount::Amount, cost::Cost, posting::Posting},
//!     config::output::OutputConfig,
//!     transaction::Transaction,
//!     writer::beancount_writer::BeancountWriter,
//! };
//! use chrono::NaiveDate;
//! use rust_decimal_macros::dec;
//!
//! let tx = Transaction::new(
//!     NaiveDate::from_ymd_opt(2024, 5, 1).expect("valid date"),
//!     "Buy fund",
//! )
//! .with_posting(
//!     Posting::new("Assets:Broker:Securities")
//!         .with_amount(Amount::new(dec!(10), "SEC_123456"))
//!         .with_cost(Cost::new(dec!(1.23), "CNY")),
//! )
//! .with_posting(Posting::new("Assets:Broker:Cash").with_amount(Amount::new(dec!(-12.3), "CNY")));
//!
//! let writer = BeancountWriter::new(OutputConfig {
//!     emit_open_directives: true,
//!     open_date: Some("2024-01-01".to_string()),
//!     ..OutputConfig::default()
//! });
//!
//! let mut output = Vec::new();
//! writer.write(&[tx], &mut output).expect("write should succeed");
//!
//! let rendered = String::from_utf8(output).expect("valid utf8");
//! assert!(rendered.contains("2024-01-01 open Assets:Broker:Cash CNY"));
//! assert!(rendered.contains("2024-01-01 commodity SEC_123456"));
//! ```

use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;

use crate::model::transaction::Transaction;

use super::{BeancountWriter, OpenAccountInfo};

impl BeancountWriter {
    /// 写出 `open` 指令。
    ///
    /// - 仅出现法币金额的账户会附带币种列表；
    /// - 出现证券/商品持仓的账户仅写账户名（可附 booking method）。
    ///
    /// # 参数
    /// - `transactions`：参与汇总的交易列表；
    /// - `writer`：目标写出流。
    ///
    /// # 返回值
    /// - `Ok(())`：写出成功或无需写出；
    /// - `Err(std::io::Error)`：底层写入失败。
    pub(super) fn write_open_directives(
        &self,
        transactions: &[Transaction],
        writer: &mut dyn std::io::Write,
    ) -> std::io::Result<()> {
        let Some(open_date) = self.resolve_open_date(transactions) else {
            return Ok(());
        };

        let accounts = self.collect_open_accounts(transactions);
        if accounts.is_empty() {
            return Ok(());
        }

        let booking_method = self.normalized_booking_method();

        for (account, info) in accounts {
            if info.has_non_fiat {
                // 非法币持仓账户不附法币列表，必要时附 booking method。
                if let Some(method) = booking_method.as_deref() {
                    writeln!(
                        writer,
                        "{} open {} \"{}\"",
                        open_date.format("%Y-%m-%d"),
                        account,
                        method
                    )?;
                } else {
                    writeln!(writer, "{} open {}", open_date.format("%Y-%m-%d"), account)?;
                }
            } else if info.fiat_currencies.is_empty() {
                // 账户仅出现了空金额过账时，保留纯账户声明。
                writeln!(writer, "{} open {}", open_date.format("%Y-%m-%d"), account)?;
            } else {
                // 法币列表按 BTreeSet 的自然序稳定输出，避免回归抖动。
                let currencies = info
                    .fiat_currencies
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                writeln!(
                    writer,
                    "{} open {} {}",
                    open_date.format("%Y-%m-%d"),
                    account,
                    currencies
                )?;
            }
        }
        writeln!(writer)?;

        Ok(())
    }

    /// 解析 `open` 指令日期。
    ///
    /// 优先使用配置中的 `open_date`，否则取最早交易日期。
    /// 若配置值格式非法（非 `%Y-%m-%d`），自动回退到最早交易日期。
    fn resolve_open_date(&self, transactions: &[Transaction]) -> Option<NaiveDate> {
        if let Some(raw) = self.config.open_date.as_deref()
            && let Ok(date) = NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d")
        {
            return Some(date);
        }

        transactions.iter().map(|tx| tx.date).min()
    }

    /// 收集需要输出 `open` 的账户及其币种信息。
    ///
    /// 账户名会先应用 `render_account`（例如前缀补全）再参与聚合。
    /// 返回 `BTreeMap` 以确保最终写出顺序稳定。
    fn collect_open_accounts(
        &self,
        transactions: &[Transaction],
    ) -> BTreeMap<String, OpenAccountInfo> {
        let mut accounts: BTreeMap<String, OpenAccountInfo> = BTreeMap::new();

        for tx in transactions {
            for posting in &tx.postings {
                let account = self.render_account(&posting.account);
                let entry = accounts.entry(account).or_default();

                if let Some(amount) = &posting.amount {
                    if Self::is_fiat_currency(&amount.currency) {
                        entry.fiat_currencies.insert(amount.currency.clone());
                    } else {
                        entry.has_non_fiat = true;
                    }
                }
            }
        }

        accounts
    }

    /// 写出 `commodity` 指令。
    ///
    /// 仅对“带 `cost` 或 `price` 的非法币 `amount.currency`”写出声明。
    ///
    /// # 参数
    /// - `transactions`：参与汇总的交易列表；
    /// - `writer`：目标写出流。
    ///
    /// # 返回值
    /// - `Ok(())`：写出成功或无需写出；
    /// - `Err(std::io::Error)`：底层写入失败。
    pub(super) fn write_commodity_directives(
        &self,
        transactions: &[Transaction],
        writer: &mut dyn std::io::Write,
    ) -> std::io::Result<()> {
        let symbols = self.collect_commodity_symbols(transactions);
        let Some(commodity_date) = self.resolve_open_date(transactions) else {
            return Ok(());
        };

        if symbols.is_empty() {
            return Ok(());
        }

        for symbol in symbols {
            // 指令使用小写并附带日期，确保语法有效。
            writeln!(
                writer,
                "{} commodity {}",
                commodity_date.format("%Y-%m-%d"),
                symbol
            )?;
        }
        writeln!(writer)?;

        Ok(())
    }

    /// 规范化 `booking_method`，仅接受 Beancount 支持值。
    ///
    /// 支持值：`STRICT`、`FIFO`、`LIFO`、`AVERAGE`、`NONE`。
    /// 输入会先去首尾空格并转为大写；非法值返回 `None`。
    fn normalized_booking_method(&self) -> Option<String> {
        let raw = self.config.booking_method.as_deref()?.trim();
        if raw.is_empty() {
            return None;
        }

        let normalized = raw.to_ascii_uppercase();
        let supported = ["STRICT", "FIFO", "LIFO", "AVERAGE", "NONE"];
        if supported.contains(&normalized.as_str()) {
            Some(normalized)
        } else {
            None
        }
    }

    /// 收集交易中需要声明的商品代码。
    ///
    /// 满足以下条件才会被收集：
    /// - 过账有 `amount`；
    /// - 同时存在 `cost` 或 `price`；
    /// - `amount.currency` 不是法币代码。
    fn collect_commodity_symbols(&self, transactions: &[Transaction]) -> BTreeSet<String> {
        let mut symbols = BTreeSet::new();

        for tx in transactions {
            for posting in &tx.postings {
                if let Some(amount) = &posting.amount
                    && (posting.cost.is_some() || posting.price.is_some())
                    && !Self::is_fiat_currency(&amount.currency)
                {
                    symbols.insert(amount.currency.clone());
                }
            }
        }

        symbols
    }

    /// 判断币种是否属于内置法币集合。
    ///
    /// 该集合用于区分“法币账户”与“商品/证券账户”，以决定
    /// `open` / `commodity` 指令渲染策略。
    fn is_fiat_currency(currency: &str) -> bool {
        matches!(
            currency,
            "CNY" | "USD" | "HKD" | "EUR" | "JPY" | "GBP" | "SGD" | "CHF" | "AUD" | "CAD"
        )
    }
}
