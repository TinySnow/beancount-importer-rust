//! 表格解析配置模型。
//!
//! 本模块定义 CSV/电子表格导入共用的解析参数。
//! 这些参数可由供应商配置覆盖，用于适配不同导出模板。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::config::tabular_options::TabularOptions;
//!
//! let options = TabularOptions::default();
//! assert_eq!(options.delimiter, ',');
//! assert_eq!(options.quote, '"');
//! assert_eq!(options.encoding, "UTF-8");
//! assert!(!options.flexible);
//! assert!(options.comment.is_none());
//! ```

use serde::{Deserialize, Serialize};

/// 表格解析选项（CSV/电子表格共用）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabularOptions {
    /// 分隔符（默认为逗号）
    #[serde(default = "default_delimiter")]
    pub delimiter: char,

    /// 引号字符
    #[serde(default = "default_quote")]
    pub quote: char,

    /// 是否允许不等长记录
    #[serde(default)]
    pub flexible: bool,

    /// 文件编码（默认 UTF-8）
    #[serde(default = "default_encoding")]
    pub encoding: String,

    /// 注释前缀
    pub comment: Option<char>,
}

impl Default for TabularOptions {
    /// 创建表格解析配置默认实例。
    fn default() -> Self {
        Self {
            delimiter: ',',
            quote: '"',
            flexible: false,
            encoding: "UTF-8".to_string(),
            comment: None,
        }
    }
}

/// `delimiter` 字段默认值工厂函数。
fn default_delimiter() -> char {
    ','
}

/// `quote` 字段默认值工厂函数。
fn default_quote() -> char {
    '"'
}

/// `encoding` 字段默认值工厂函数。
fn default_encoding() -> String {
    "UTF-8".to_string()
}
