use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;

use crate::model::transaction::Transaction;

use super::{BeancountWriter, OpenAccountInfo};

impl BeancountWriter {
    /// 写出 `open` 指令。
    ///
    /// - 仅出现法币金额的账户会附带币种列表；
    /// - 出现证券/商品持仓的账户仅写账户名（可附 booking method）。
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
                writeln!(writer, "{} open {}", open_date.format("%Y-%m-%d"), account)?;
            } else {
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
    fn resolve_open_date(&self, transactions: &[Transaction]) -> Option<NaiveDate> {
        if let Some(raw) = self.config.open_date.as_deref()
            && let Ok(date) = NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d")
        {
            return Some(date);
        }

        transactions.iter().map(|tx| tx.date).min()
    }

    /// 收集需要输出 `open` 的账户及其币种信息。
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

    /// 规范化 booking method，只接受 Beancount 支持值。
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

    /// 判断币种是否属于法币集合。
    fn is_fiat_currency(currency: &str) -> bool {
        matches!(
            currency,
            "CNY" | "USD" | "HKD" | "EUR" | "JPY" | "GBP" | "SGD" | "CHF" | "AUD" | "CAD"
        )
    }
}
