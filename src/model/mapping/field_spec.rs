//! 字段规格模型。
//!
//! 本模块定义了单个字段映射的两种表达方式：
//! - 简写：直接写列名字符串；
//! - 详写：写成对象，并附带默认值、转换器和正则提取规则。
//!
//! `FieldSpec` 通过 `#[serde(untagged)]` 同时兼容这两种 YAML 语法，
//! 便于在配置文件中按需增量升级。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::mapping::field_spec::FieldSpec;
//!
//! let simple: FieldSpec = serde_yaml::from_str(r#""交易金额""#).unwrap();
//! assert_eq!(simple.column_name(), "交易金额");
//! assert_eq!(simple.default_value(), None);
//!
//! let detailed_yaml = r#"
//! column: "交易金额"
//! default: "0"
//! transform: abs
//! regex_extract: "([0-9.]+)"
//! "#;
//! let detailed: FieldSpec = serde_yaml::from_str(detailed_yaml).unwrap();
//! assert_eq!(detailed.column_name(), "交易金额");
//! assert_eq!(detailed.default_value(), Some("0"));
//! assert_eq!(detailed.transformer(), Some("abs"));
//! assert_eq!(detailed.regex_extract_pattern(), Some("([0-9.]+)"));
//! ```

use serde::{Deserialize, Serialize};

/// 单个字段的取值规格。
///
/// 该枚举兼容两种 YAML 表达形式：
/// - `Simple("金额")`：只指定列名；
/// - `Detailed(...)`：同时指定列名、默认值和转换策略。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldSpec {
    /// 简写配置：仅指定列名。
    Simple(String),
    /// 详写配置：可附带默认值、转换和正则提取。
    Detailed(DetailedFieldSpec),
}

impl FieldSpec {
    /// 获取配置中声明的源列名。
    ///
    /// # 返回值
    /// 返回用于读取输入字段的列名。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::mapping::field_spec::FieldSpec;
    ///
    /// let spec: FieldSpec = serde_yaml::from_str(r#""交易对方""#).unwrap();
    /// assert_eq!(spec.column_name(), "交易对方");
    /// ```
    pub fn column_name(&self) -> &str {
        match self {
            FieldSpec::Simple(name) => name,
            FieldSpec::Detailed(spec) => &spec.column,
        }
    }

    /// 获取字段默认值（若配置了 `default`）。
    ///
    /// 简写形式不支持默认值，因此始终返回 `None`。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::mapping::field_spec::FieldSpec;
    ///
    /// let spec: FieldSpec = serde_yaml::from_str(
    ///     r#"
    /// column: "币种"
    /// default: "CNY"
    /// "#,
    /// )
    /// .unwrap();
    /// assert_eq!(spec.default_value(), Some("CNY"));
    /// ```
    pub fn default_value(&self) -> Option<&str> {
        match self {
            FieldSpec::Simple(_) => None,
            FieldSpec::Detailed(spec) => spec.default.as_deref(),
        }
    }

    /// 获取数值转换器名称（若配置了 `transform`）。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::mapping::field_spec::FieldSpec;
    ///
    /// let spec: FieldSpec = serde_yaml::from_str(
    ///     r#"
    /// column: "金额"
    /// transform: negate
    /// "#,
    /// )
    /// .unwrap();
    /// assert_eq!(spec.transformer(), Some("negate"));
    /// ```
    pub fn transformer(&self) -> Option<&str> {
        match self {
            FieldSpec::Simple(_) => None,
            FieldSpec::Detailed(spec) => spec.transform.as_deref(),
        }
    }

    /// 获取正则提取表达式（若配置了 `regex_extract`）。
    ///
    /// 当设置该字段时，读取流程会对原始文本做正则匹配，
    /// 优先使用第一个捕获组，否则使用完整匹配结果。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::mapping::field_spec::FieldSpec;
    ///
    /// let spec: FieldSpec = serde_yaml::from_str(
    ///     r#"
    /// column: "摘要"
    /// regex_extract: "订单号[:：]?\\s*(\\d+)"
    /// "#,
    /// )
    /// .unwrap();
    /// assert_eq!(spec.regex_extract_pattern(), Some("订单号[:：]?\\s*(\\d+)"));
    /// ```
    pub fn regex_extract_pattern(&self) -> Option<&str> {
        match self {
            FieldSpec::Simple(_) => None,
            FieldSpec::Detailed(spec) => spec.regex_extract.as_deref(),
        }
    }
}

/// 详写字段配置。
///
/// 该结构用于在“列名”之外扩展更复杂的映射行为，
/// 例如为空时回退默认值、数值转换、正则抽取。
///
/// # 示例
/// ```rust
/// use beancount_importer_rust::model::mapping::field_spec::DetailedFieldSpec;
///
/// let spec = DetailedFieldSpec {
///     column: "交易金额".to_string(),
///     default: Some("0".to_string()),
///     transform: Some("abs".to_string()),
///     regex_extract: None,
/// };
/// assert_eq!(spec.column, "交易金额");
/// assert_eq!(spec.default.as_deref(), Some("0"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedFieldSpec {
    /// 源数据中的列名。
    pub column: String,
    /// 默认值（当字段缺失或为空时使用）。
    pub default: Option<String>,
    /// 转换器名称（例如 `negate`、`abs`）。
    pub transform: Option<String>,
    /// 正则提取表达式（优先返回第一个捕获组，否则返回完整匹配）。
    pub regex_extract: Option<String>,
}
