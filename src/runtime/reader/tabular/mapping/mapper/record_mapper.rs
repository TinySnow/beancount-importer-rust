//! 单行字段到 `RawRecord` 的映射。
//!
//! 本模块负责把字段映射配置应用到单条表格行：
//! - 14 个标准字段按映射规则读取；
//! - `extra_fields` 的兼容性映射；
//! - 常见银行导出表头的兜底推断；
//! - `payTime` / `type` 等扩展字段规范化。

use std::collections::HashMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::{
    error::ImporterResult,
    model::{
        data::raw_record::RawRecord,
        mapping::field_mapping::FieldMapping,
    },
    utils::time::normalize_time_text,
};

use crate::runtime::reader::tabular::TabularRecordReader;

impl TabularRecordReader {
    /// 将单行字段映射为 `RawRecord`。
    ///
    /// 当 `mapping` 缺失时，会把所有非空列直接写入 `record.extra`。
    pub(super) fn map_to_raw_record(
        &self,
        fields: &HashMap<String, String>,
        mapping: Option<&FieldMapping>,
    ) -> ImporterResult<RawRecord> {
        let mut record = RawRecord::new();

        let Some(mapping) = mapping else {
            for (key, value) in fields {
                if !value.is_empty() {
                    record.extra.insert(key.clone(), value.clone());
                }
            }
            return Ok(record);
        };

        // 标准字段优先按显式映射读取。
        record.date = self.map_date(fields, mapping.date.as_ref(), &mapping.date_formats)?;
        record.amount = self.map_decimal(fields, mapping.amount.as_ref())?;
        record.currency = self.map_text(fields, mapping.currency.as_ref())?;
        record.payee = self.map_text(fields, mapping.payee.as_ref())?;
        record.narration = self.map_text(fields, mapping.narration.as_ref())?;
        record.transaction_type = self.map_text(fields, mapping.transaction_type.as_ref())?;
        record.status = self.map_text(fields, mapping.status.as_ref())?;
        record.reference = self.map_text(fields, mapping.reference.as_ref())?;
        record.symbol = self.map_text(fields, mapping.symbol.as_ref())?;
        record.security_name = self.map_text(fields, mapping.security_name.as_ref())?;
        record.quantity = self.map_decimal(fields, mapping.quantity.as_ref())?;
        record.unit_price = self.map_decimal(fields, mapping.unit_price.as_ref())?;
        record.fee = self.map_decimal(fields, mapping.fee.as_ref())?;
        record.tax = self.map_decimal(fields, mapping.tax.as_ref())?;

        // 然后补充扩展字段和兼容性兜底推断。
        self.map_extra_fields(fields, mapping, &mut record);
        self.apply_common_fallbacks(fields, mapping, &mut record);
        self.normalize_pay_time_extra(&mut record);
        self.fill_direction_type_extra(&mut record);

        Ok(record)
    }

    /// 把标准方向字段同步到 `extra.type`，便于规则按 `field: type` 匹配。
    ///
    /// 仅在 `extra.type` 缺失时回填，不覆盖显式映射值。
    fn fill_direction_type_extra(&self, record: &mut RawRecord) {
        if record.extra.contains_key("type") {
            return;
        }

        let Some(direction) = record
            .transaction_type
            .as_deref()
            .and_then(Self::normalize_direction_text)
        else {
            return;
        };

        record
            .extra
            .insert("type".to_string(), direction.to_string());
    }

    /// 统一方向语义文本，收敛到"收入/支出"二元值。
    ///
    /// 该函数同时兼容中英文关键字，避免不同上游数据源在方向字段上出现歧义。
    fn normalize_direction_text(raw: &str) -> Option<&'static str> {
        if raw.contains("支出") || raw.contains("转出") {
            return Some("支出");
        }
        if raw.contains("收入") || raw.contains("转入") {
            return Some("收入");
        }

        let normalized = raw.trim().to_ascii_lowercase();
        if normalized.contains("expense") {
            return Some("支出");
        }
        if normalized.contains("income") {
            return Some("收入");
        }

        None
    }

    /// 规范化 `payTime` 元数据，统一输出 `HH:MM:SS`。
    ///
    /// 支持输入：
    /// - Excel 序列时间（如 `46110.56767361111`）
    /// - 日期时间字符串（如 `2026-03-06 14:37:15`）
    /// - 时间字符串（如 `14:37` / `14:37:15`）
    fn normalize_pay_time_extra(&self, record: &mut RawRecord) {
        let Some(raw) = record.extra.get("payTime").cloned() else {
            return;
        };

        if let Some(normalized) = normalize_time_text(&raw) {
            record.extra.insert("payTime".to_string(), normalized);
        }
    }

    /// 对常见银行导出列做兜底推断，提升不同表头变体的兼容性。
    fn apply_common_fallbacks(
        &self,
        fields: &HashMap<String, String>,
        mapping: &FieldMapping,
        record: &mut RawRecord,
    ) {
        if record.date.is_none() {
            record.date = self.infer_date_from_common_columns(fields, &mapping.date_formats);
        }

        if record.payee.is_none() {
            record.payee = self.first_non_empty_text(
                fields,
                &["交易对方", "对方户名", "对手户名", "payee", "counterparty"],
            );
        }

        if record.reference.is_none() {
            record.reference = self.first_non_empty_text(
                fields,
                &["交易流水号", "流水号", "reference", "orderId", "交易单号"],
            );
        }

        if record.currency.is_none() {
            record.currency = self.first_non_empty_text(fields, &["币种", "currency", "Currency"]);
        }

        if record.amount.is_none() || record.transaction_type.is_none() {
            let (amount, direction) = self.infer_split_amount_and_direction(fields);
            if record.amount.is_none() {
                record.amount = amount;
            }
            if record.transaction_type.is_none() {
                record.transaction_type = direction;
            }
        }
    }

    /// 从常见日期列兜底提取交易日期。
    fn infer_date_from_common_columns(
        &self,
        fields: &HashMap<String, String>,
        formats: &[String],
    ) -> Option<NaiveDate> {
        const DATE_KEYS: [&str; 6] = [
            "交易日期",
            "记账日",
            "入账日期",
            "date",
            "transaction_date",
            "booking_date",
        ];

        for key in DATE_KEYS {
            let Some(value) = self.non_empty_value(fields.get(key).map(String::as_str)) else {
                continue;
            };
            if let Some(parsed) = self.parse_date(value, formats) {
                return Some(parsed);
            }
        }

        None
    }

    /// 读取"支出/收入"分列结构并推断金额与方向。
    fn infer_split_amount_and_direction(
        &self,
        fields: &HashMap<String, String>,
    ) -> (Option<Decimal>, Option<String>) {
        const EXPENSE_KEYS: [&str; 8] = [
            "支出",
            "出账金额",
            "出账",
            "借方金额",
            "借方发生额",
            "debit",
            "debit_amount",
            "withdrawal",
        ];
        const INCOME_KEYS: [&str; 8] = [
            "收入",
            "入账金额",
            "入账",
            "贷方金额",
            "贷方发生额",
            "credit",
            "credit_amount",
            "deposit",
        ];
        const EXPENSE_VARIANTS: [&str; 4] = [
            "交易金额(支出)",
            "交易金额（支出）",
            "记账金额(支出)",
            "记账金额（支出）",
        ];
        const INCOME_VARIANTS: [&str; 4] = [
            "交易金额(收入)",
            "交易金额（收入）",
            "记账金额(收入)",
            "记账金额（收入）",
        ];

        let expense = self
            .first_non_empty_decimal(fields, &EXPENSE_KEYS)
            .or_else(|| self.first_non_empty_decimal(fields, &EXPENSE_VARIANTS));
        let income = self
            .first_non_empty_decimal(fields, &INCOME_KEYS)
            .or_else(|| self.first_non_empty_decimal(fields, &INCOME_VARIANTS));

        let expense = expense.filter(|value| !value.is_zero());
        let income = income.filter(|value| !value.is_zero());

        match (expense, income) {
            (Some(value), None) => (Some(value), Some("支出".to_string())),
            (None, Some(value)) => (Some(value), Some("收入".to_string())),
            (Some(exp), Some(inc)) => {
                let net = inc - exp;
                if net.is_zero() {
                    (Some(exp), Some("支出".to_string()))
                } else if net.is_sign_positive() {
                    (Some(net), Some("收入".to_string()))
                } else {
                    (Some(net.abs()), Some("支出".to_string()))
                }
            }
            (None, None) => (None, None),
        }
    }

    /// 映射 `extra_fields` 并兼容历史配置方向。
    ///
    /// 推荐写法为 `extra_key -> source_column`，同时兼容旧写法
    /// `source_column -> extra_key`，以减少历史配置迁移成本。
    fn map_extra_fields(
        &self,
        fields: &HashMap<String, String>,
        mapping: &FieldMapping,
        record: &mut RawRecord,
    ) {
        for (left, right) in &mapping.extra_fields {
            if let Some(value) = self.non_empty_value(fields.get(right).map(String::as_str)) {
                record.extra.insert(left.clone(), value.to_string());
            } else if let Some(value) = self.non_empty_value(fields.get(left).map(String::as_str)) {
                record.extra.insert(right.clone(), value.to_string());
            }
        }
    }
}
