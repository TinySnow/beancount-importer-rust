//! 銆愭ā鍧楁枃妗ｃ€?
//!
//! 妯″潡鍚嶇О锛氭簮鐮佹ā鍧?
//! 鏂囦欢璺緞锛?
//! 鏍稿績鑱岃矗锛氭壙鎷呭綋鍓嶆枃浠跺搴旂殑鍔熻兘瀹炵幇
//! 涓昏杈撳叆锛氫笂娓告ā鍧椾紶鍏ョ殑鏁版嵁
//! 涓昏杈撳嚭锛氫笅娓告ā鍧楁秷璐圭殑鏁版嵁鎴栬涓?
//! 缁存姢璇存槑锛氬彉鏇村墠搴旂‘璁ゅ叾鍦ㄥ鍏ラ摼璺腑鐨勪綅缃笌褰卞搷
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

/// Beancount 鏍煎紡鍐欏嚭鍣ㄣ€?
pub struct BeancountWriter {
    config: OutputConfig,
}

impl BeancountWriter {
    /// 鍒涘缓鍐欏嚭鍣ㄣ€?
    pub fn new(config: OutputConfig) -> Self {
        Self { config }
    }

    /// 灏嗕氦鏄撳啓鍑轰负 Beancount 鏍煎紡銆?
    pub fn write(
        &self,
        transactions: &[Transaction],
        writer: &mut dyn Write,
    ) -> std::io::Result<()> {
        // 鍙€夊啓鍏?`open` 鎸囦护锛岀‘淇濈嫭绔?Beancount 鏂囦欢鍙洿鎺ユ牎楠屻€?
        if self.config.emit_open_directives {
            self.write_open_directives(transactions, writer)?;
        }

        // 鍦ㄤ氦鏄撳墠鍏堝０鏄?`commodity`锛岄伩鍏嶆湭澹版槑鍟嗗搧瀵艰嚧瑙ｆ瀽鍛婅銆?
        self.write_commodity_directives(transactions, writer)?;

        // 鏈€鍚庢寜椤哄簭鍐欏嚭姣忕瑪浜ゆ槗銆?
        for (index, tx) in transactions.iter().enumerate() {
            if index > 0 {
                writeln!(writer)?;
            }
            self.write_transaction(tx, writer)?;
        }

        Ok(())
    }

    /// 鍐欏嚭 `open` 鎸囦护銆?
    ///
    /// 鑻ヨ处鎴蜂粎鍑虹幇娉曞竵閲戦锛屽垯杩藉姞鍙敤甯佺鍒楄〃锛?
    /// 鑻ュ惈璇佸埜/鍟嗗搧鎸佷粨锛屽垯浠呭啓璐︽埛鍚嶃€?
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

    /// 瑙ｆ瀽 `open` 鎸囦护鏃ユ湡銆?
    ///
    /// 浼樺厛浣跨敤閰嶇疆椤?`open_date`锛屽惁鍒欏彇鏈€鏃╀氦鏄撴棩鏈熴€?
    fn resolve_open_date(&self, transactions: &[Transaction]) -> Option<NaiveDate> {
        if let Some(raw) = self.config.open_date.as_deref() {
            if let Ok(date) = NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d") {
                return Some(date);
            }
        }

        transactions.iter().map(|tx| tx.date).min()
    }

    /// 鎵弿浜ゆ槗锛屾敹闆嗛渶瑕?`open` 鐨勮处鎴峰強鍏跺竵绉嶄俊鎭€?
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

    /// 鍐欏嚭 `commodity` 鎸囦护銆?
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
            // Use lowercase `commodity` directive and include date for valid syntax.
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

    /// 浠庤繃璐︿腑鏀堕泦闈炴硶甯佸晢鍝佷唬鐮併€?
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

    /// 鍐欏嚭涓€绗斾氦鏄撱€?
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

    /// 鍐欏嚭涓€鏉¤繃璐︺€?
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

    /// 鎸夎緭鍑洪厤缃覆鏌撹处鎴峰悕锛堝彲鑷姩琛ュ墠缂€锛夈€?
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

    /// 鎸夐敭鎺掑簭杈撳嚭鍏冩暟鎹紝淇濊瘉缁撴灉绋冲畾銆?
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

    /// 鎸夐厤缃簿搴︽牸寮忓寲鍗佽繘鍒舵暟鍊笺€?
    fn format_decimal(&self, value: rust_decimal::Decimal) -> String {
        format!(
            "{:.prec$}",
            value,
            prec = self.config.decimal_places as usize
        )
    }

    /// 瑙勮寖鍖栨棩鏈熸牸寮忓瓧绗︿覆锛屽幓鎺夊灞傚紩鍙枫€?
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
    /// Renders tags and links appended to the transaction header line.
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
    /// 杞箟瀛楃涓蹭腑鐨勫弽鏂滄潬鍜屽弻寮曞彿锛岄伩鍏?Beancount 璇硶閿欒銆?
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
                .with_amount(Amount::new(dec!(-10), "SEC_123456"))
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
        assert!(result.contains("-10.00 SEC_123456 {} @ 1.23 CNY"));
    }

    #[test]
    fn test_open_directives_are_emitted_when_enabled() {
        let tx = Transaction::new(
            NaiveDate::from_ymd_opt(2024, 5, 1).expect("valid date"),
            "Buy fund",
        )
        .with_posting(
            Posting::new("Assets:Broker:Securities")
                .with_amount(Amount::new(dec!(10), "SEC_123456"))
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
                .with_amount(Amount::new(dec!(10), "SEC_123456"))
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
                .with_amount(Amount::new(dec!(10), "SEC_123456"))
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
        assert!(result.contains("2024-05-01 commodity SEC_123456"));
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
