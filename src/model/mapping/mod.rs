//! 字段映射模型模块。
//!
//! 该模块用于描述“源数据列 -> 标准字段”的映射配置，主要包含两层抽象：
//! - [`field_spec::FieldSpec`]：单个字段如何取值（简写列名或详写配置）。
//! - [`field_mapping::FieldMapping`]：完整记录的标准字段映射集合。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::mapping::field_mapping::FieldMapping;
//!
//! let yaml = r#"
//! date: "交易日期"
//! amount:
//!   column: "交易金额"
//!   transform: abs
//! "#;
//!
//! let mapping: FieldMapping = serde_yaml::from_str(yaml).unwrap();
//! assert_eq!(
//!     mapping.get_standard_mapping("date").unwrap().column_name(),
//!     "交易日期"
//! );
//! assert_eq!(
//!     mapping.get_standard_mapping("amount").unwrap().transformer(),
//!     Some("abs")
//! );
//! ```

pub mod field_mapping;
pub mod field_spec;
