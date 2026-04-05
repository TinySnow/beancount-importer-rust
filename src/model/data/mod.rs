//! 原始导入数据模型模块。
//!
//! 该模块用于承载“读取器解析后、规则映射前”的中间结构。
//! 当前包含 [`raw_record`] 子模块，用于描述一条标准化原始记录。
//!
//! # 典型用法
//! ```rust
//! use beancount_importer_rust::model::data::raw_record::RawRecord;
//!
//! let record = RawRecord::new();
//! assert!(record.date.is_none());
//! ```

pub mod raw_record;
