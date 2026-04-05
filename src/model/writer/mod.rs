//! 写出器模块。
//!
//! 该模块负责将标准交易模型渲染为 Beancount 文本，
//! 并导出当前默认写出实现 [`beancount_writer`]。
//!
//! # 主要能力
//! - 按配置控制日期格式与小数位数；
//! - 可选写出 `open` / `commodity` 指令；
//! - 输出稳定排序的 metadata，便于回归对比。
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
//!     NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid date"),
//!     "Lunch",
//! )
//! .with_posting(Posting::new("Expenses:Food").with_amount(Amount::new(dec!(10), "CNY")))
//! .with_posting(Posting::new("Assets:Cash").with_amount(Amount::new(dec!(-10), "CNY")));
//!
//! let writer = BeancountWriter::new(OutputConfig::default());
//! let mut output = Vec::new();
//! writer.write(&[tx], &mut output).expect("write should succeed");
//!
//! let rendered = String::from_utf8(output).expect("valid utf8");
//! assert!(rendered.contains("2024-01-01 * \"Lunch\""));
//! ```

pub mod beancount_writer;
