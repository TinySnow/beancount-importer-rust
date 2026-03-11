//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
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
