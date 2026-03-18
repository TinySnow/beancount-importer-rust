use std::collections::HashMap;

use log::trace;

use crate::{
    model::{account::posting::Posting, config::meta_value::MetaValue, transaction::Transaction},
    utils::metadata::ensure_beancount_metadata_key,
};

use super::BeancountWriter;

impl BeancountWriter {
    /// 写出单笔交易。
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
    fn write_sorted_metadata(
        &self,
        metadata: &HashMap<String, MetaValue>,
        indent: &str,
        writer: &mut dyn std::io::Write,
    ) -> std::io::Result<()> {
        let mut entries: Vec<_> = metadata.iter().collect();
        entries.sort_by(|left, right| left.0.cmp(right.0));

        for (key, value) in entries {
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
