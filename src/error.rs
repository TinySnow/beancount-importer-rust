//! 统一错误类型模块
//!
//! 该模块定义了导入流程的统一错误枚举 [`ImporterError`] 与结果别名 [`ImporterResult`]，
//! 用于在配置加载、记录解析、字段映射、规则匹配和数据转换等环节保持一致的错误处理接口。
//!
//! # 主要功能
//! - 统一声明导入器各阶段的业务错误
//! - 封装第三方库和标准库错误类型
//! - 通过 `#[from]` 支持 `?` 自动错误转换
//! - 提供统一的 `ImporterResult<T>` 返回类型
//!
//! # 关键类型
//! - [`ImporterError`]：导入器统一错误枚举
//! - [`ImporterResult`]：统一结果类型别名
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::error::{ImporterError, ImporterResult};
//!
//! fn parse_line(line: &str, line_no: usize) -> ImporterResult<i32> {
//!     line.parse::<i32>().map_err(|_| ImporterError::Parse {
//!         line: line_no,
//!         message: format!("invalid integer: {line}"),
//!     })
//! }
//!
//! assert_eq!(parse_line("42", 1).unwrap(), 42);
//! assert!(matches!(
//!     parse_line("oops", 2),
//!     Err(ImporterError::Parse { line: 2, .. })
//! ));
//! ```
//!
//! ```rust
//! use beancount_importer_rust::error::{ImporterError, ImporterResult};
//!
//! fn io_step() -> ImporterResult<()> {
//!     let io_result: Result<(), std::io::Error> =
//!         Err(std::io::Error::new(std::io::ErrorKind::Other, "disk unavailable"));
//!     io_result?;
//!     Ok(())
//! }
//!
//! assert!(matches!(io_step(), Err(ImporterError::Io(_))));
//! ```

use thiserror::Error;

/**
 * 导入器统一错误类型
 *
 * 该枚举将导入流程中可能出现的业务错误与依赖库错误统一抽象为同一类型，
 * 便于调用方在边界层统一打印、分类和上抛错误。
 *
 * # 设计要点
 * - 业务错误使用结构化字段，便于精准定位问题来源
 * - 外部错误变体通过 `#[from]` 支持 `?` 自动转换
 * - 每个变体都提供清晰、可读的错误消息格式
 *
 * # 示例
 * ```rust
 * use beancount_importer_rust::error::ImporterError;
 *
 * let err = ImporterError::FieldMapping {
 *     field: "amount".to_string(),
 * };
 *
 * assert_eq!(
 *     err.to_string(),
 *     "Field mapping error: field 'amount' not found in record"
 * );
 * ```
 */
#[derive(Error, Debug)]
pub enum ImporterError {
    /// 配置文件相关错误
    #[error("Configuration error: {0}")]
    Config(String),

    /// 解析错误
    #[error("Parse error at line {line}: {message}")]
    Parse {
        /// 发生解析错误的行号（从 1 开始计数）
        line: usize,
        /// 解析失败的具体原因
        message: String,
    },

    /// 字段映射错误
    #[error("Field mapping error: field '{field}' not found in record")]
    FieldMapping {
        /// 在输入记录中未找到的字段名
        field: String,
    },

    /// 规则匹配错误
    #[error("Rule matching error: {0}")]
    RuleMatch(String),

    /// 数据转换错误
    #[error("Data conversion error: {0}")]
    Conversion(String),

    /// 供应商未找到
    #[error("Provider '{0}' not found")]
    ProviderNotFound(String),

    /// IO 错误（通过 `?` 自动由 `std::io::Error` 转换）
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// CSV 解析错误（通过 `?` 自动由 `csv::Error` 转换）
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    /// YAML 解析错误（通过 `?` 自动由 `serde_yaml::Error` 转换）
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// 正则表达式错误（通过 `?` 自动由 `regex::Error` 转换）
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    /// 日期解析错误（通过 `?` 自动由 `chrono::ParseError` 转换）
    #[error("Date parse error: {0}")]
    DateParse(#[from] chrono::ParseError),

    /// 数值解析错误（通过 `?` 自动由 `rust_decimal::Error` 转换）
    #[error("Decimal parse error: {0}")]
    DecimalParse(#[from] rust_decimal::Error),

    /// 未分类的内部错误，用于包裹带运行时上下文的既有错误。
    #[error("{0}")]
    Internal(String),
}

impl ImporterError {
    /// 创建未分类的内部错误。
    ///
    /// 通常用于包裹带运行时上下文的既有错误，或在无法匹配
    /// 具体业务变体时作为兜底。
    pub fn internal(msg: impl Into<String>) -> Self {
        ImporterError::Internal(msg.into())
    }

    /// 为当前错误附加一层上游上下文，包裹为 [`Internal`](ImporterError::Internal)。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::error::ImporterError;
    ///
    /// let inner = ImporterError::Io(std::io::Error::new(
    ///     std::io::ErrorKind::NotFound,
    ///     "file missing",
    /// ));
    /// let wrapped = inner.with_context("Failed to load mapping");
    ///
    /// assert!(matches!(wrapped, ImporterError::Internal(_)));
    /// assert_eq!(
    ///     wrapped.to_string(),
    ///     "Failed to load mapping: IO error: file missing"
    /// );
    /// ```
    pub fn with_context(self, msg: impl Into<String>) -> Self {
        ImporterError::Internal(format!("{}: {self}", msg.into()))
    }
}

/**
 * 导入器结果类型别名
 *
 * 该别名用于统一导入器 API 的返回类型，减少重复书写 `Result<T, ImporterError>`，
 * 并让函数签名更聚焦于业务返回值本身。
 *
 * # 示例
 * ```rust
 * use beancount_importer_rust::error::{ImporterError, ImporterResult};
 *
 * fn validate_provider(provider: &str) -> ImporterResult<()> {
 *     if provider.trim().is_empty() {
 *         return Err(ImporterError::ProviderNotFound("<empty>".to_string()));
 *     }
 *     Ok(())
 * }
 *
 * assert!(validate_provider("icbc").is_ok());
 * assert!(matches!(
 *     validate_provider(""),
 *     Err(ImporterError::ProviderNotFound(_))
 * ));
 * ```
 */
pub type ImporterResult<T> = Result<T, ImporterError>;
