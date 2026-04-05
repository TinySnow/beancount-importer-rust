//! 数据读取层入口。
//!
//! `reader` 模块负责把不同来源的输入文件读取为统一的中间记录，
//! 供后续规则引擎和写出器继续处理。
//!
//! 当前已实现的读取器：
//! - [`tabular`]：面向 CSV/XLS/XLSX 的表格读取器。
//!
//! # 示例
//! ```rust,no_run
//! use std::path::Path;
//!
//! use beancount_importer_rust::{
//!     model::config::tabular_options::TabularOptions,
//!     runtime::reader::tabular::TabularRecordReader,
//! };
//!
//! let reader = TabularRecordReader::new(
//!     TabularOptions::default(),
//!     0,
//!     true,
//!     false,
//! );
//! let _records = reader.read_file(Path::new("statement.csv"), None)?;
//! # Ok::<(), beancount_importer_rust::error::ImporterError>(())
//! ```

pub mod tabular;
