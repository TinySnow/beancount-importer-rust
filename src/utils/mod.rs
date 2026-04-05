//! 通用工具模块。
//!
//! 该模块聚合导入流程中复用的通用能力，包括：
//! - 币种规范化；
//! - 日期/时间解析；
//! - 数值文本解析；
//! - 文件编码读取；
//! - 元数据键名归一化；
//! - 日志初始化与文本辅助判断。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::utils::{currency, decimal};
//!
//! assert_eq!(currency::normalize_cash_currency(Some("人民币")), "CNY");
//! assert_eq!(decimal::parse_decimal("¥1,234.56").is_some(), true);
//! ```

pub mod currency;
pub mod date;
pub mod decimal;
pub mod encoding;
pub mod init;
pub mod metadata;
pub mod text;
pub mod time;
