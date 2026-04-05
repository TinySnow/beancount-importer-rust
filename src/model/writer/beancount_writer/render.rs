//! 交易与过账渲染逻辑。
//!
//! 本模块负责把 `Transaction` 与 `Posting` 渲染为 Beancount 文本行，
//! 并处理以下细节：
//! - 标题行中的 `payee` / `narration` / tags / links；
//! - 过账行中的金额、成本与价格格式；
//! - metadata 键名规范化与稳定排序输出；
//! - 日期格式与字符串转义。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::{
//!     account::{amount::Amount, posting::Posting},
//!     config::output::OutputConfig,
//!     transaction::Transaction,
//!     writer::beancount_writer::BeancountWriter,
//! };
//! use chrono::NaiveDate;
//! use rust_decimal_macros::dec;
//!
//! let tx = Transaction::new(
//!     NaiveDate::from_ymd_opt(2024, 6, 1).expect("valid date"),
//!     "Tagged transaction",
//! )
//! .with_payee("Payee")
//! .with_tag("food")
//! .with_link("order123")
//! .with_posting(Posting::new("Expenses:Food").with_amount(Amount::new(dec!(10), "CNY")))
//! .with_posting(Posting::new("Assets:Cash").with_amount(Amount::new(dec!(-10), "CNY")));
//!
//! let writer = BeancountWriter::new(OutputConfig::default());
//! let mut output = Vec::new();
//! writer.write(&[tx], &mut output).expect("write should succeed");
//!
//! let rendered = String::from_utf8(output).expect("valid utf8");
//! assert!(rendered.contains("\"Payee\" \"Tagged transaction\" #food ^order123"));
//! assert!(rendered.contains("Expenses:Food  10.00 CNY"));
//! ```

use std::collections::HashMap;

use log::trace;

use crate::{
    model::{account::posting::Posting, config::meta_value::MetaValue, transaction::Transaction},
    utils::metadata::ensure_beancount_metadata_key,
};

use super::BeancountWriter;

impl BeancountWriter {
    /// 写出单笔交易。
    ///
    /// # 参数
    /// - `tx`：待写出的交易；
    /// - `writer`：目标写出流。
    ///
    /// # 返回值
    /// - `Ok(())`：写出成功；
    /// - `Err(std::io::Error)`：底层写入失败。
    pub(super) fn write_transaction(
        &self,
        tx: &Transaction,
        writer: &mut dyn std::io::Write,
    ) -> std::io::Result<()> {
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

    /// 写出一条过账分录。
    ///
    /// 该函数会依次写出：账户、金额、成本（含推断成本）与价格，
    /// 然后输出该过账关联的 metadata。
    fn write_posting(
        &self,
        posting: &Posting,
        writer: &mut dyn std::io::Write,
    ) -> std::io::Result<()> {
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

        // `{}` 与 `{<cost>}` 互斥：推断成本优先于显式成本。
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

    /// 根据输出配置渲染账户名（支持前缀补全）。
    ///
    /// 若配置了 `account_prefix` 且账户不以该前缀开头，
    /// 则输出 `<prefix>:<account>`。
    pub(super) fn render_account(&self, account: &str) -> String {
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

    /// 按键名排序写出 metadata，保证输出稳定。
    ///
    /// # 参数
    /// - `metadata`：待写出的键值对；
    /// - `indent`：每行前缀缩进；
    /// - `writer`：目标写出流。
    fn write_sorted_metadata(
        &self,
        metadata: &HashMap<String, MetaValue>,
        indent: &str,
        writer: &mut dyn std::io::Write,
    ) -> std::io::Result<()> {
        let mut entries: Vec<_> = metadata.iter().collect();
        entries.sort_by(|left, right| left.0.cmp(right.0));

        for (key, value) in entries {
            // Beancount metadata key 必须满足标识符规则。
            let normalized_key = ensure_beancount_metadata_key(key);
            writeln!(writer, "{}{}: {}", indent, normalized_key, value)?;
        }

        Ok(())
    }

    /// 按配置精度格式化十进制金额。
    fn format_decimal(&self, value: rust_decimal::Decimal) -> String {
        format!(
            "{:.prec$}",
            value,
            prec = self.config.decimal_places as usize
        )
    }

    /// 规范化日期格式字符串，去除外层引号。
    ///
    /// 例如：`"%Y-%m-%d"` 会转换为 `%Y-%m-%d`，
    /// 以兼容用户在 YAML 中误加引号的配置值。
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

    /// 渲染交易头部后缀的 tags 与 links。
    ///
    /// 输出格式：` #tag1 #tag2 ^link1 ^link2`。
    /// 空白或空字符串条目会被忽略。
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

    /// 转义字符串中的反斜杠与双引号，避免语法冲突。
    fn escape_string(&self, raw: &str) -> String {
        raw.replace('\\', "\\\\").replace('"', "\\\"")
    }
}
