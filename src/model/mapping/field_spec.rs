//! 模块说明：字段映射模型定义，用于源数据列到标准字段的映射。
//!
//! 文件路径：src/model/mapping/field_spec.rs。
//! 该文件围绕 'field_spec' 的职责提供实现。
//! 关键符号：FieldSpec、column_name、default_value、transformer。

use serde::{Deserialize, Serialize};

/// 字段规格
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldSpec {
    /// 简写：仅指定列名
    Simple(String),
    /// 详写：可附带默认值、转换和正则提取
    Detailed(DetailedFieldSpec),
}

impl FieldSpec {
    /// 获取列名
    pub fn column_name(&self) -> &str {
        match self {
            FieldSpec::Simple(name) => name,
            FieldSpec::Detailed(spec) => &spec.column,
        }
    }

    /// 获取默认值
    pub fn default_value(&self) -> Option<&str> {
        match self {
            FieldSpec::Simple(_) => None,
            FieldSpec::Detailed(spec) => spec.default.as_deref(),
        }
    }

    /// 获取转换器名称
    pub fn transformer(&self) -> Option<&str> {
        match self {
            FieldSpec::Simple(_) => None,
            FieldSpec::Detailed(spec) => spec.transform.as_deref(),
        }
    }

    /// 获取正则提取表达式
    pub fn regex_extract_pattern(&self) -> Option<&str> {
        match self {
            FieldSpec::Simple(_) => None,
            FieldSpec::Detailed(spec) => spec.regex_extract.as_deref(),
        }
    }
}

/// 详细字段配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedFieldSpec {
    /// CSV 列名
    pub column: String,
    /// 默认值（字段为空时使用）
    pub default: Option<String>,
    /// 转换器名称（例如 `negate`, `abs`）
    pub transform: Option<String>,
    /// 正则提取（优先返回第一个捕获组，否则返回完整匹配）
    pub regex_extract: Option<String>,
}
