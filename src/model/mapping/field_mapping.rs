//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
//! 源表头到标准原始记录字段的映射配置。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::model::mapping::field_spec::FieldSpec;

/// 源列与标准原始记录字段之间的映射。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FieldMapping {
    pub date: Option<FieldSpec>,
    pub amount: Option<FieldSpec>,
    pub currency: Option<FieldSpec>,
    pub payee: Option<FieldSpec>,
    pub narration: Option<FieldSpec>,
    pub transaction_type: Option<FieldSpec>,
    pub status: Option<FieldSpec>,
    pub reference: Option<FieldSpec>,

    // 证券相关字段。
    pub symbol: Option<FieldSpec>,
    pub security_name: Option<FieldSpec>,
    pub quantity: Option<FieldSpec>,
    pub unit_price: Option<FieldSpec>,
    pub fee: Option<FieldSpec>,
    pub tax: Option<FieldSpec>,

    /// 额外元数据键 -> CSV 列名。
    ///
    /// `csv_reader` 同时支持旧版反向格式（`csv_column -> extra_key`）
    /// 以保持向后兼容。
    #[serde(default)]
    pub extra_fields: HashMap<String, String>,

    /// 日期解析格式列表。
    #[serde(default = "default_date_formats")]
    pub date_formats: Vec<String>,
}

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
    /// 按字段名获取一个标准映射。
    pub fn get_standard_mapping(&self, field_name: &str) -> Option<&FieldSpec> {
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
