//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    io::Write,
};

use chrono::NaiveDate;
use log::trace;

use crate::{
    model::{
        account::posting::Posting, config::meta_value::MetaValue, config::output::OutputConfig,
        transaction::Transaction,
    },
    utils::metadata::ensure_beancount_metadata_key,
};

#[derive(Debug, Default)]
struct OpenAccountInfo {
    fiat_currencies: BTreeSet<String>,
    has_non_fiat: bool,
}

/// Beancount 格式写出器。
pub struct BeancountWriter {
    config: OutputConfig,
}

impl BeancountWriter {
    /// 创建写出器。
    pub fn new(config: OutputConfig) -> Self {
        Self { config }
    }

    /// 将交易写出为 Beancount 格式。
    pub fn write(
        &self,
        transactions: &[Transaction],
        writer: &mut dyn Write,
    ) -> std::io::Result<()> {
        // 可选写入 `open` 指令，确保独立 Beancount 文件可直接校验。
        if self.config.emit_open_directives {
            self.write_open_directives(transactions, writer)?;
        }

        // 在交易前先声明 `commodity`，避免未声明商品导致解析告警。
        self.write_commodity_directives(transactions, writer)?;

        // 最后按顺序写出每笔交易。
        for (index, tx) in transactions.iter().enumerate() {
            if index > 0 {
                writeln!(writer)?;
            }
            self.write_transaction(tx, writer)?;
        }

        Ok(())
    }

    /// 写出 `open` 指令。
    ///
    /// 若账户仅出现法币金额，则追加可用币种列表；
    /// 若含证券/商品持仓，则仅写账户名。
    fn write_open_directives(
        &self,
        transactions: &[Transaction],
        writer: &mut dyn Write,
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
    /// 优先使用配置项 `open_date`，否则取最早交易日期。
    fn resolve_open_date(&self, transactions: &[Transaction]) -> Option<NaiveDate> {
        if let Some(raw) = self.config.open_date.as_deref() {
            if let Ok(date) = NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d") {
                return Some(date);
            }
        }

        transactions.iter().map(|tx| tx.date).min()
    }

    /// 扫描交易，收集需要 `open` 的账户及其币种信息。
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
    fn write_commodity_directives(
        &self,
        transactions: &[Transaction],
        writer: &mut dyn Write,
    ) -> std::io::Result<()> {
        let symbols = self.collect_commodity_symbols(transactions);
        let Some(commodity_date) = self.resolve_open_date(transactions) else {
            return Ok(());
        };

        if symbols.is_empty() {
            return Ok(());
        }

        for symbol in symbols {
            // 使用小写 `commodity` 指令，并携带日期，满足 Beancount 语法要求。
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

    /// 从过账中收集非法币商品代码。
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

    fn is_fiat_currency(currency: &str) -> bool {
        matches!(
            currency,
            "CNY" | "USD" | "HKD" | "EUR" | "JPY" | "GBP" | "SGD" | "CHF" | "AUD" | "CAD"
        )
    }

    /// 写出一笔交易。
    fn write_transaction(&self, tx: &Transaction, writer: &mut dyn Write) -> std::io::Result<()> {
        trace!("Writing transaction: {:?}", tx);

        let date_format = Self::sanitize_date_format(&self.config.date_format);
        write!(writer, "{} {}", tx.date.format(date_format), tx.flag)?;
        let tags_links = Self::render_tags_links(tx);

        match (&tx.payee, &tx.narration) {
            (Some(payee), narration) => {
                writeln!(
                    writer,
                    " \"{}\" \"{}\"{}",
                    self.escape_string(payee),
                    self.escape_string(narration),
                    tags_links
                )?;
            }
            (None, narration) => {
                writeln!(
                    writer,
                    " \"{}\"{}",
                    self.escape_string(narration),
                    tags_links
                )?;
            }
        }

        self.write_sorted_metadata(&tx.metadata, "  ", writer)?;

        for posting in &tx.postings {
            self.write_posting(posting, writer)?;
        }

        Ok(())
    }

    /// 写出一条过账。
    fn write_posting(&self, posting: &Posting, writer: &mut dyn Write) -> std::io::Result<()> {
        let account = self.render_account(&posting.account);

        write!(writer, "  ")?;

        if let Some(flag) = posting.flag {
            write!(writer, "{} ", flag)?;
        }

        write!(writer, "{}", account)?;

        if let Some(amount) = &posting.amount {
            let formatted_number = self.format_decimal(amount.number);
            write!(writer, "  {} {}", formatted_number, amount.currency)?;
        }

        if posting.inferred_cost {
            write!(writer, " {{}}")?;
        } else if let Some(cost) = &posting.cost {
            write!(writer, " {{{}}}", cost)?;
        }

        if let Some(price) = &posting.price {
            write!(writer, " @ {}", price)?;
        }

        writeln!(writer)?;

        self.write_sorted_metadata(&posting.metadata, "    ", writer)?;

        Ok(())
    }

    /// 按输出配置渲染账户名（可自动补前缀）。
    fn render_account(&self, account: &str) -> String {
        if let Some(prefix) = &self.config.account_prefix {
            if account.starts_with(prefix) {
                account.to_string()
            } else {
                format!("{}:{}", prefix, account)
            }
        } else {
            account.to_string()
        }
    }

    /// 按键排序输出元数据，保证结果稳定。
    fn write_sorted_metadata(
        &self,
        metadata: &HashMap<String, MetaValue>,
        indent: &str,
        writer: &mut dyn Write,
    ) -> std::io::Result<()> {
        let mut entries: Vec<_> = metadata.iter().collect();
        entries.sort_by(|left, right| left.0.cmp(right.0));

        for (key, value) in entries {
            let normalized_key = ensure_beancount_metadata_key(key);
            writeln!(writer, "{}{}: {}", indent, normalized_key, value)?;
        }

        Ok(())
    }

    /// 按配置精度格式化十进制数值。
    fn format_decimal(&self, value: rust_decimal::Decimal) -> String {
        format!(
            "{:.prec$}",
            value,
            prec = self.config.decimal_places as usize
        )
    }

    /// 规范化日期格式字符串，去掉外层引号。
    fn sanitize_date_format(raw: &str) -> &str {
        let trimmed = raw.trim();
        if trimmed.len() >= 2 {
            let first = trimmed.as_bytes()[0] as char;
            let last = trimmed.as_bytes()[trimmed.len() - 1] as char;
            if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
                return &trimmed[1..trimmed.len() - 1];
            }
        }
        trimmed
    }
    /// 渲染交易头行的 tags/links。
    fn render_tags_links(tx: &Transaction) -> String {
        let mut parts = Vec::new();

        for tag in &tx.tags {
            let normalized = tag.trim();
            if !normalized.is_empty() {
                parts.push(format!("#{}", normalized));
            }
        }

        for link in &tx.links {
            let normalized = link.trim();
            if !normalized.is_empty() {
                parts.push(format!("^{}", normalized));
            }
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!(" {}", parts.join(" "))
        }
    }
    /// 转义字符串中的反斜杠和双引号，避免 Beancount 语法错误。
    fn escape_string(&self, raw: &str) -> String {
        raw.replace('\\', "\\\\").replace('"', "\\\"")
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    use super::*;
    use crate::model::account::{amount::Amount, cost::Cost, price::Price};

    #[test]
    fn test_simple_transaction() {
        let tx = Transaction::new(
            NaiveDate::from_ymd_opt(2024, 1, 15).expect("valid date"),
            "Coffee at Starbucks",
        )
        .with_payee("Starbucks")
        .with_posting(
            Posting::new("Expenses:Food:Coffee").with_amount(Amount::new(dec!(35.00), "CNY")),
        )
        .with_posting(Posting::new("Assets:Cash"));

        let writer = BeancountWriter::new(OutputConfig::default());
        let mut output = Vec::new();
        {
            let mut cursor = std::io::Cursor::new(&mut output);
            writer
                .write(&[tx], &mut cursor)
                .expect("writer should succeed");
        }

        let result = String::from_utf8(output).expect("utf8 output");
        assert!(result.contains("2024-01-15 * \"Starbucks\" \"Coffee at Starbucks\""));
        assert!(result.contains("Expenses:Food:Coffee  35.00 CNY"));
        assert!(result.contains("Assets:Cash"));
    }

    #[test]
    fn test_quoted_date_format_is_sanitized() {
        let tx = Transaction::new(
            NaiveDate::from_ymd_opt(2024, 3, 1).expect("valid date"),
            "Quoted date format",
        )
        .with_posting(Posting::new("Assets:Cash"));

        let config = OutputConfig {
            date_format: "\"%Y-%m-%d\"".to_string(),
            ..OutputConfig::default()
        };
        let writer = BeancountWriter::new(config);

        let mut output = Vec::new();
        {
            let mut cursor = std::io::Cursor::new(&mut output);
            writer
                .write(&[tx], &mut cursor)
                .expect("writer should succeed");
        }

        let result = String::from_utf8(output).expect("utf8 output");
        assert!(result.starts_with("2024-03-01 *"));
        assert!(!result.starts_with("\"2024-03-01\" *"));
    }

    #[test]
    fn test_inferred_cost_posting_is_rendered_as_empty_braces() {
        let tx = Transaction::new(
            NaiveDate::from_ymd_opt(2024, 3, 2).expect("valid date"),
            "Sell security",
        )
        .with_posting(
            Posting::new("Assets:Broker:Securities")
                .with_amount(Amount::new(dec!(-10), "FUND_123456"))
                .with_inferred_cost()
                .with_price(Price::new(dec!(1.23), "CNY")),
        )
        .with_posting(
            Posting::new("Assets:Broker:Cash").with_amount(Amount::new(dec!(12.30), "CNY")),
        )
        .with_posting(Posting::new("Income:Investing:Capital-Gains"));

        let writer = BeancountWriter::new(OutputConfig::default());
        let mut output = Vec::new();
        {
            let mut cursor = std::io::Cursor::new(&mut output);
            writer
                .write(&[tx], &mut cursor)
                .expect("writer should succeed");
        }

        let result = String::from_utf8(output).expect("utf8 output");
        assert!(result.contains("-10.00 FUND_123456 {} @ 1.23 CNY"));
    }

    #[test]
    fn test_open_directives_are_emitted_when_enabled() {
        let tx = Transaction::new(
            NaiveDate::from_ymd_opt(2024, 5, 1).expect("valid date"),
            "Buy fund",
        )
        .with_posting(
            Posting::new("Assets:Broker:Securities")
                .with_amount(Amount::new(dec!(10), "FUND_123456"))
                .with_cost(Cost::new(dec!(1.23), "CNY")),
        )
        .with_posting(
            Posting::new("Assets:Broker:Cash").with_amount(Amount::new(dec!(-12.30), "CNY")),
        )
        .with_posting(
            Posting::new("Expenses:Investing:Fees").with_amount(Amount::new(dec!(0.10), "CNY")),
        );

        let config = OutputConfig {
            emit_open_directives: true,
            open_date: Some("2024-01-01".to_string()),
            ..OutputConfig::default()
        };
        let writer = BeancountWriter::new(config);

        let mut output = Vec::new();
        {
            let mut cursor = std::io::Cursor::new(&mut output);
            writer
                .write(&[tx], &mut cursor)
                .expect("writer should succeed");
        }

        let result = String::from_utf8(output).expect("utf8 output");
        assert!(result.contains("2024-01-01 open Assets:Broker:Cash CNY"));
        assert!(result.contains("2024-01-01 open Assets:Broker:Securities"));
        assert!(result.contains("2024-01-01 open Expenses:Investing:Fees CNY"));
    }

    #[test]
    fn test_open_directives_include_booking_method_for_non_fiat_accounts() {
        let tx = Transaction::new(
            NaiveDate::from_ymd_opt(2024, 5, 1).expect("valid date"),
            "Buy fund",
        )
        .with_posting(
            Posting::new("Assets:Broker:Securities")
                .with_amount(Amount::new(dec!(10), "FUND_123456"))
                .with_cost(Cost::new(dec!(1.23), "CNY")),
        )
        .with_posting(
            Posting::new("Assets:Broker:Cash").with_amount(Amount::new(dec!(-12.30), "CNY")),
        );

        let config = OutputConfig {
            emit_open_directives: true,
            open_date: Some("2024-01-01".to_string()),
            booking_method: Some("fifo".to_string()),
            ..OutputConfig::default()
        };
        let writer = BeancountWriter::new(config);

        let mut output = Vec::new();
        {
            let mut cursor = std::io::Cursor::new(&mut output);
            writer
                .write(&[tx], &mut cursor)
                .expect("writer should succeed");
        }

        let result = String::from_utf8(output).expect("utf8 output");
        assert!(result.contains("2024-01-01 open Assets:Broker:Securities \"FIFO\""));
        assert!(result.contains("2024-01-01 open Assets:Broker:Cash CNY"));
    }

    #[test]
    fn test_commodity_directive_uses_date_and_lowercase_keyword() {
        let tx = Transaction::new(
            NaiveDate::from_ymd_opt(2024, 5, 1).expect("valid date"),
            "Buy fund",
        )
        .with_posting(
            Posting::new("Assets:Broker:Securities")
                .with_amount(Amount::new(dec!(10), "FUND_123456"))
                .with_cost(Cost::new(dec!(1.23), "CNY")),
        )
        .with_posting(
            Posting::new("Assets:Broker:Cash").with_amount(Amount::new(dec!(-12.30), "CNY")),
        );

        let writer = BeancountWriter::new(OutputConfig::default());
        let mut output = Vec::new();
        {
            let mut cursor = std::io::Cursor::new(&mut output);
            writer
                .write(&[tx], &mut cursor)
                .expect("writer should succeed");
        }

        let result = String::from_utf8(output).expect("utf8 output");
        assert!(result.contains("2024-05-01 commodity FUND_123456"));
        assert!(!result.contains("COMMODITY"));
    }

    #[test]
    fn test_tags_and_links_are_emitted_on_header_line() {
        let tx = Transaction::new(
            NaiveDate::from_ymd_opt(2024, 6, 1).expect("valid date"),
            "Tagged transaction",
        )
        .with_payee("Payee")
        .with_tag("food")
        .with_tag("lunch")
        .with_link("order123")
        .with_posting(Posting::new("Expenses:Food").with_amount(Amount::new(dec!(10), "CNY")))
        .with_posting(Posting::new("Assets:Cash").with_amount(Amount::new(dec!(-10), "CNY")));

        let writer = BeancountWriter::new(OutputConfig::default());
        let mut output = Vec::new();
        {
            let mut cursor = std::io::Cursor::new(&mut output);
            writer
                .write(&[tx], &mut cursor)
                .expect("writer should succeed");
        }

        let result = String::from_utf8(output).expect("utf8 output");
        assert!(result.contains("\"Payee\" \"Tagged transaction\" #food #lunch ^order123"));
        assert!(!result.contains("; Tags:"));
        assert!(!result.contains("; Links:"));
    }
}
