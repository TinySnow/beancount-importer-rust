//! 模块说明：统一错误类型与 Result 别名定义。
//!
//! 文件路径：src/error.rs。
//! 该文件围绕 'error' 的职责提供实现。
//! 关键符号：ImporterError、ImporterResult。

use thiserror::Error;

/// 导入器错误类型
#[derive(Error, Debug)]
pub enum ImporterError {
    /// 配置文件相关错误
    #[error("Configuration error: {0}")]
    Config(String),

    /// 解析错误
    #[error("Parse error at line {line}: {message}")]
    Parse { line: usize, message: String },

    /// 字段映射错误
    #[error("Field mapping error: field '{field}' not found in record")]
    FieldMapping { field: String },

    /// 规则匹配错误
    #[error("Rule matching error: {0}")]
    RuleMatch(String),

    /// 数据转换错误
    #[error("Data conversion error: {0}")]
    Conversion(String),

    /// 供应商未找到
    #[error("Provider '{0}' not found")]
    ProviderNotFound(String),

    /// IO 错误
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// CSV 解析错误
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    /// YAML 解析错误
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// 正则表达式错误
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    /// 日期解析错误
    #[error("Date parse error: {0}")]
    DateParse(#[from] chrono::ParseError),

    /// 数值解析错误
    #[error("Decimal parse error: {0}")]
    DecimalParse(#[from] rust_decimal::Error),
}

/// 导入器结果类型别名
pub type ImporterResult<T> = Result<T, ImporterError>;
