//! Beancount 写出器实现。
//!
//! 本模块定义 [`BeancountWriter`]，负责把标准化 `Transaction`
//! 集合写出为可直接落盘的 Beancount 文本。
//!
//! # 输出阶段
//! 1. （可选）写出 `open` 指令；
//! 2. 写出 `commodity` 指令；
//! 3. 逐笔写出交易主体与过账。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::{
//!     account::{amount::Amount, posting::Posting},
//!     config::output::OutputConfig,
//!     transaction::Transaction,
//! };
//! use beancount_importer_rust::runtime::writer::beancount_writer::BeancountWriter;
//! use chrono::NaiveDate;
//! use rust_decimal_macros::dec;
//!
//! let tx = Transaction::new(
//!     NaiveDate::from_ymd_opt(2024, 1, 15).expect("valid date"),
//!     "Coffee at Starbucks",
//! )
//! .with_payee("Starbucks")
//! .with_posting(Posting::new("Expenses:Food:Coffee").with_amount(Amount::new(dec!(35), "CNY")))
//! .with_posting(Posting::new("Assets:Cash"));
//!
//! let writer = BeancountWriter::new(OutputConfig::default());
//! let mut output = Vec::new();
//! writer.write(&[tx], &mut output).expect("write should succeed");
//!
//! let rendered = String::from_utf8(output).expect("valid utf8");
//! assert!(rendered.contains("2024-01-15 * \"Starbucks\" \"Coffee at Starbucks\""));
//! assert!(rendered.contains("Expenses:Food:Coffee  35.00 CNY"));
//! ```

mod directives;
mod render;

#[cfg(test)]
mod tests;

use std::{collections::BTreeSet, io::Write};

use crate::model::{config::output::OutputConfig, transaction::Transaction};

/// 账户在 `open` 指令收集阶段的聚合信息。
#[derive(Debug, Default)]
struct OpenAccountInfo {
    /// 该账户出现过的法币币种集合（去重并保持有序）。
    fiat_currencies: BTreeSet<String>,
    /// 该账户是否出现过非标准法币商品（如基金/证券代码）。
    has_non_fiat: bool,
}

/// Beancount 文本写出器。
///
/// 该类型持有输出配置，并提供将交易集合写出到任意 `Write`
/// 目标的统一入口。
pub struct BeancountWriter {
    /// 当前写出流程使用的配置。
    config: OutputConfig,
}

impl BeancountWriter {
    /// 创建写出器。
    ///
    /// # 参数
    /// - `config`：控制日期格式、精度与额外指令输出的配置。
    ///
    /// # 返回值
    /// - 新的 [`BeancountWriter`] 实例。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::config::output::OutputConfig;
    /// use beancount_importer_rust::runtime::writer::beancount_writer::BeancountWriter;
    ///
    /// let writer = BeancountWriter::new(OutputConfig::default());
    /// let _ = writer;
    /// ```
    pub fn new(config: OutputConfig) -> Self {
        Self { config }
    }

    /// 按配置把交易集合写出为 Beancount 文本。
    ///
    /// 输出顺序：
    /// 1. （可选）`open` 指令；
    /// 2. `commodity` 指令；
    /// 3. 逐笔交易。
    ///
    /// # 参数
    /// - `transactions`：待写出的交易切片；
    /// - `writer`：输出目标（文件、内存缓冲区等）。
    ///
    /// # 返回值
    /// - `Ok(())`：全部写出成功；
    /// - `Err(std::io::Error)`：底层写入失败。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::{
    ///     account::{amount::Amount, posting::Posting},
    ///     config::output::OutputConfig,
    ///     transaction::Transaction,
    /// };
    /// use beancount_importer_rust::runtime::writer::beancount_writer::BeancountWriter;
    /// use chrono::NaiveDate;
    /// use rust_decimal_macros::dec;
    ///
    /// let tx = Transaction::new(
    ///     NaiveDate::from_ymd_opt(2024, 6, 1).expect("valid date"),
    ///     "Lunch",
    /// )
    /// .with_posting(Posting::new("Expenses:Food").with_amount(Amount::new(dec!(12), "CNY")))
    /// .with_posting(Posting::new("Assets:Cash").with_amount(Amount::new(dec!(-12), "CNY")));
    ///
    /// let writer = BeancountWriter::new(OutputConfig::default());
    /// let mut output = Vec::new();
    /// writer.write(&[tx], &mut output).expect("write should succeed");
    ///
    /// let rendered = String::from_utf8(output).expect("valid utf8");
    /// assert!(rendered.contains("Expenses:Food  12.00 CNY"));
    /// ```
    pub fn write(
        &self,
        transactions: &[Transaction],
        writer: &mut dyn Write,
    ) -> std::io::Result<()> {
        if self.config.emit_open_directives {
            self.write_open_directives(transactions, writer)?;
        }

        self.write_commodity_directives(transactions, writer)?;

        for (index, tx) in transactions.iter().enumerate() {
            // 交易之间留一个空行，保证与常见 Beancount 风格一致。
            if index > 0 {
                writeln!(writer)?;
            }
            self.write_transaction(tx, writer)?;
        }

        Ok(())
    }
}
