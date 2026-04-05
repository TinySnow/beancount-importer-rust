//! 交易模型模块。
//!
//! 该模块封装 Beancount 交易（`transaction`）的核心数据结构与构建 API，
//! 当前由 [`entry`] 子模块提供 [`Transaction`] 类型。
//!
//! # 设计说明
//! - `entry`：定义交易实体与交易平衡校验逻辑。
//! - `pub use entry::Transaction`：对外提供简洁导入路径，避免调用方感知内部文件拆分。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::transaction::Transaction;
//! use chrono::NaiveDate;
//!
//! let tx = Transaction::new(
//!     NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(),
//!     "Sample transaction",
//! );
//!
//! assert_eq!(tx.narration, "Sample transaction");
//! assert_eq!(tx.flag, '*');
//! ```

pub mod entry;

pub use entry::Transaction;
