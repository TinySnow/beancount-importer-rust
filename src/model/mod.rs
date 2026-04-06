//! 核心领域模型总入口。
//!
//! `model` 模块定义导入流程中使用的核心数据结构，并按领域职责拆分子模块，
//! 为读取、规则匹配、交易构建与输出写入提供统一的数据边界。
//!
//! # 子模块概览
//! - [`account`]：金额、成本、价格、过账项等会计基础对象。
//! - [`transaction`]：交易分录模型与构建接口。
//! - [`rule`]：规则定义、条件运算、匹配模式与执行引擎。
//! - [`mapping`]：字段映射模型（原始字段到领域字段的映射规范）。
//! - [`data`]：原始输入记录及中间数据结构。
//! - [`config`]：全局与供应商配置模型。
//!
//! # 设计约束
//! - 该层以“可序列化、可组合、可测试”的纯模型为主，避免运行时副作用。
//! - 运行时编排位于 `runtime` 层，`model` 仅承担数据表达与约束职责。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::{
//!     account::amount::Amount,
//!     config::global::GlobalConfig,
//!     transaction::Transaction,
//! };
//! use chrono::NaiveDate;
//! use rust_decimal::Decimal;
//!
//! let amount = Amount::new(Decimal::new(12345, 2), "CNY");
//! assert_eq!(amount.currency, "CNY");
//!
//! let tx = Transaction::new(
//!     NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(),
//!     "Doc example",
//! );
//! assert_eq!(tx.flag, '*');
//!
//! let global = GlobalConfig::default();
//! assert_eq!(global.default_currency, "CNY");
//! ```

/// 账户与过账基础模型（金额、成本、价格、过账项）。
pub mod account;
/// 导入流程配置模型（全局、供应商、输出、元数据类型）。
pub mod config;
/// 输入数据与原始记录模型。
pub mod data;
/// 字段映射模型。
pub mod mapping;
/// 规则模型与执行引擎。
pub mod rule;
/// 交易分录模型。
pub mod transaction;
