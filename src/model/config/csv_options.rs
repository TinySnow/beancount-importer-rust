//! 模块说明：配置模型定义与序列化反序列化规则。
//!
//! 文件路径：src/model/config/csv_options.rs。
//! 该文件围绕 'csv_options' 的职责提供实现。
//! 关键符号：CsvOptions、default、default_delimiter、default_quote。

use serde::{Deserialize, Serialize};

/// CSV 解析选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvOptions {
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

impl Default for CsvOptions {
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

fn default_delimiter() -> char {
    ','
}

fn default_quote() -> char {
    '"'
}

fn default_encoding() -> String {
    "UTF-8".to_string()
}
