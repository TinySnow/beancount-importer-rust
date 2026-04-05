//! 字段映射集合模型。
//!
//! 本模块提供 [`FieldMapping`]，用于描述一条“原始记录”里各标准字段
//! 应该从源数据的哪一列读取，以及读取时的补充策略。
//!
//! 映射中的每个字段都使用 [`FieldSpec`](crate::model::mapping::field_spec::FieldSpec)
//! 表达，因此同一份配置可以混合简写和详写语法。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::mapping::field_mapping::FieldMapping;
//!
//! let yaml = r#"
//! date: "交易时间"
//! amount:
//!   column: "金额"
//!   transform: abs
//! extra_fields:
//!   source_file: "来源文件"
//! "#;
//!
//! let mapping: FieldMapping = serde_yaml::from_str(yaml).unwrap();
//! assert_eq!(
//!     mapping.get_standard_mapping("date").unwrap().column_name(),
//!     "交易时间"
//! );
//! assert_eq!(
//!     mapping.get_standard_mapping("amount").unwrap().transformer(),
//!     Some("abs")
//! );
//! assert_eq!(mapping.extra_fields.get("source_file"), Some(&"来源文件".to_string()));
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::model::mapping::field_spec::FieldSpec;

/// 源列与标准原始记录字段之间的映射。
///
/// 所有标准字段均为可选项，便于按数据源逐步配置。
/// 未声明的字段在读取时会保持 `None`。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FieldMapping {
    /// 交易日期字段映射。
    pub date: Option<FieldSpec>,
    /// 交易金额字段映射。
    pub amount: Option<FieldSpec>,
    /// 币种字段映射。
    pub currency: Option<FieldSpec>,
    /// 交易对方字段映射。
    pub payee: Option<FieldSpec>,
    /// 摘要/说明字段映射。
    pub narration: Option<FieldSpec>,
    /// 交易类型字段映射（如收入、支出、买入、卖出）。
    pub transaction_type: Option<FieldSpec>,
    /// 交易状态字段映射（如已清算、待处理）。
    pub status: Option<FieldSpec>,
    /// 外部参考号字段映射。
    pub reference: Option<FieldSpec>,

    /// 证券代码字段映射。
    pub symbol: Option<FieldSpec>,
    /// 证券名称字段映射。
    pub security_name: Option<FieldSpec>,
    /// 持仓数量字段映射。
    pub quantity: Option<FieldSpec>,
    /// 单价字段映射。
    pub unit_price: Option<FieldSpec>,
    /// 手续费字段映射。
    pub fee: Option<FieldSpec>,
    /// 税费字段映射。
    pub tax: Option<FieldSpec>,

    /// 额外元数据映射，推荐写法为 `extra_key -> csv_column`。
    ///
    /// 读取器同时兼容旧版反向写法（`csv_column -> extra_key`），
    /// 便于平滑迁移历史配置。
    #[serde(default)]
    pub extra_fields: HashMap<String, String>,

    /// 日期解析格式列表。
    ///
    /// 当配置中未显式设置时，使用 [`default_date_formats`]。
    #[serde(default = "default_date_formats")]
    pub date_formats: Vec<String>,
}

/// 提供默认日期格式列表。
///
/// 这些格式覆盖了常见的日期与日期时间写法，
/// 作为缺省配置用于解析 `date` 字段。
fn default_date_formats() -> Vec<String> {
    vec![
        "%Y-%m-%d".to_string(),
        "%Y/%m/%d".to_string(),
        "%Y-%m-%d %H:%M:%S".to_string(),
        "%Y/%m/%d %H:%M:%S".to_string(),
        "%Y/%m/%d %H:%M".to_string(),
    ]
}

impl FieldMapping {
    /// 按标准字段名获取映射定义。
    ///
    /// 该函数只处理内置标准字段，不解析 `extra_fields`。
    ///
    /// # 参数
    /// - `field_name`：标准字段名，如 `date`、`amount`、`payee`。
    ///
    /// # 返回值
    /// - `Some(&FieldSpec)`：配置了该字段映射；
    /// - `None`：字段名不受支持或未配置映射。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::mapping::field_mapping::FieldMapping;
    ///
    /// let yaml = r#"
    /// date: "交易日期"
    /// amount: "交易金额"
    /// "#;
    /// let mapping: FieldMapping = serde_yaml::from_str(yaml).unwrap();
    ///
    /// assert_eq!(
    ///     mapping.get_standard_mapping("amount").unwrap().column_name(),
    ///     "交易金额"
    /// );
    /// assert!(mapping.get_standard_mapping("unknown").is_none());
    /// ```
    pub fn get_standard_mapping(&self, field_name: &str) -> Option<&FieldSpec> {
        // 显式匹配而非动态查找，保持标准字段集合稳定且可审计。
        match field_name {
            "date" => self.date.as_ref(),
            "amount" => self.amount.as_ref(),
            "currency" => self.currency.as_ref(),
            "payee" => self.payee.as_ref(),
            "narration" => self.narration.as_ref(),
            "transaction_type" => self.transaction_type.as_ref(),
            "status" => self.status.as_ref(),
            "reference" => self.reference.as_ref(),
            "symbol" => self.symbol.as_ref(),
            "security_name" => self.security_name.as_ref(),
            "quantity" => self.quantity.as_ref(),
            "unit_price" => self.unit_price.as_ref(),
            "fee" => self.fee.as_ref(),
            "tax" => self.tax.as_ref(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FieldMapping;

    #[test]
    fn supports_shorthand_string_syntax() {
        let yaml = r#"
payee: "交易对方"
amount: "金额"
"#;

        let mapping: FieldMapping =
            serde_yaml::from_str(yaml).expect("shorthand mapping should parse");

        let payee = mapping
            .payee
            .as_ref()
            .expect("payee mapping should exist")
            .column_name();
        let amount = mapping
            .amount
            .as_ref()
            .expect("amount mapping should exist")
            .column_name();

        assert_eq!(payee, "交易对方");
        assert_eq!(amount, "金额");
    }

    #[test]
    fn supports_detailed_object_syntax() {
        let yaml = r#"
amount:
  column: "金额"
  transform: abs
"#;

        let mapping: FieldMapping =
            serde_yaml::from_str(yaml).expect("detailed mapping should parse");

        let amount = mapping
            .amount
            .as_ref()
            .expect("amount mapping should exist");

        assert_eq!(amount.column_name(), "金额");
        assert_eq!(amount.transformer(), Some("abs"));
    }

    #[test]
    fn supports_mixed_syntax_in_one_file() {
        let yaml = r#"
date: "交易时间"
amount:
  column: "金额"
  transform: abs
payee: "交易对方"
"#;

        let mapping: FieldMapping = serde_yaml::from_str(yaml).expect("mixed mapping should parse");

        assert_eq!(
            mapping.date.as_ref().expect("date mapping").column_name(),
            "交易时间"
        );
        assert_eq!(
            mapping.payee.as_ref().expect("payee mapping").column_name(),
            "交易对方"
        );
        assert_eq!(
            mapping
                .amount
                .as_ref()
                .expect("amount mapping")
                .transformer(),
            Some("abs")
        );
    }
}
