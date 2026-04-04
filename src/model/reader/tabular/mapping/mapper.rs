//! 模块说明：CSV/XLS 源读取与字段映射解析能力。
//!
//! 文件路径：src/model/reader/tabular/mapping/mapper.rs。
//! 该文件围绕字段映射职责提供实现。
//! 关键符号：validate_mapping、map_to_raw_record、map_date、map_decimal。

use std::collections::HashMap;

use chrono::{NaiveDate, NaiveDateTime};
use log::{info, trace, warn};
use regex::Regex;
use rust_decimal::Decimal;

use crate::{
    error::{ImporterError, ImporterResult},
    model::{
        data::raw_record::RawRecord,
        mapping::{field_mapping::FieldMapping, field_spec::FieldSpec},
    },
    utils::{
        decimal::parse_decimal_with_transform,
        time::{normalize_time_text, parse_excel_serial_date},
    },
};

use crate::model::reader::tabular::{TabularRecordReader, table::TabularData};

impl TabularRecordReader {
    /// 将表格行映射为标准 `RawRecord` 列表。
    pub(in crate::model::reader::tabular) fn map_table_to_records(
        &self,
        table: TabularData,
        mapping: Option<&FieldMapping>,
    ) -> ImporterResult<Vec<RawRecord>> {
        if let Some(mapping) = mapping {
            self.validate_mapping(mapping, &table.headers);
        }

        let expected_columns = table.headers.len();
        let mut records = Vec::new();
        let mut mapping_errors = 0usize;

        for mut row in table.rows {
            if Self::is_blank_row(&row.cells) || Self::is_summary_row(&row.cells) {
                continue;
            }

            // 某些银行 CSV 数据行会在末尾追加一个分隔符，导致“多 1 个空列”。
            // 这里裁掉超出的尾部空列，避免把格式噪音当成结构错误。
            while row.cells.len() > expected_columns
                && row
                    .cells
                    .last()
                    .map(|value| value.trim().is_empty())
                    .unwrap_or(false)
            {
                row.cells.pop();
            }

            if row.cells.len() != expected_columns {
                warn!(
                    "Line {}: field count mismatch (expected {}, got {})",
                    row.line_no,
                    expected_columns,
                    row.cells.len()
                );

                if self.strict_mode {
                    return Err(ImporterError::Parse {
                        line: row.line_no,
                        message: format!(
                            "Field count mismatch (expected {}, got {})",
                            expected_columns,
                            row.cells.len()
                        ),
                    });
                }
            }

            let field_map = table
                .headers
                .iter()
                .zip(row.cells.iter())
                .map(|(header, value)| (header.clone(), normalize_cell_value(value)))
                .collect::<HashMap<_, _>>();

            match self.map_to_raw_record(&field_map, mapping) {
                Ok(record) => records.push(record),
                Err(error) => {
                    mapping_errors += 1;
                    warn!("Line {}: mapping error - {}", row.line_no, error);

                    if self.strict_mode {
                        return Err(ImporterError::Parse {
                            line: row.line_no,
                            message: format!("Mapping error: {error}"),
                        });
                    }
                }
            }
        }

        let total_errors = table.pre_parse_errors + mapping_errors;
        info!(
            "{} parsing complete: {} records parsed, {} errors",
            table.source_name,
            records.len(),
            total_errors
        );

        Ok(records)
    }

    /// 是否为空白行。
    fn is_blank_row(cells: &[String]) -> bool {
        cells.iter().all(|value| value.trim().is_empty())
    }

    /// 是否为汇总/合计尾行。
    fn is_summary_row(cells: &[String]) -> bool {
        let Some(first_non_empty) = cells.iter().find_map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }) else {
            return false;
        };

        first_non_empty.contains("合计")
            || first_non_empty.eq_ignore_ascii_case("total")
            || first_non_empty.eq_ignore_ascii_case("subtotal")
    }

    /// 校验 mapping 中引用的列名是否存在于表头。
    fn validate_mapping(&self, mapping: &FieldMapping, headers: &[String]) {
        for (name, spec) in Self::mapped_specs(mapping) {
            if let Some(spec) = spec {
                let column = spec.column_name();
                if headers.iter().any(|header| header == column) {
                    trace!("Mapping '{}' -> '{}'", name, column);
                } else {
                    warn!(
                        "Mapping field '{}' references column '{}' that is not in source headers",
                        name, column
                    );
                }
            }
        }
    }

    fn map_to_raw_record(
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

    /// 读取“支出/收入”分列结构并推断金额与方向。
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

    /// 从候选键中取第一个非空文本值。
    fn first_non_empty_text(
        &self,
        fields: &HashMap<String, String>,
        keys: &[&str],
    ) -> Option<String> {
        keys.iter().find_map(|key| {
            self.non_empty_value(fields.get(*key).map(String::as_str))
                .map(str::to_string)
        })
    }

    /// 从候选键中取第一个可解析的非空金额值（绝对值）。
    fn first_non_empty_decimal(
        &self,
        fields: &HashMap<String, String>,
        keys: &[&str],
    ) -> Option<Decimal> {
        keys.iter().find_map(|key| {
            let value = self.non_empty_value(fields.get(*key).map(String::as_str))?;
            parse_decimal_with_transform(value, Some("abs"))
        })
    }

    /// 映射日期字段，按配置格式逐个尝试解析。
    fn map_date(
        &self,
        fields: &HashMap<String, String>,
        spec: Option<&FieldSpec>,
        formats: &[String],
    ) -> ImporterResult<Option<NaiveDate>> {
        let Some(spec) = spec else {
            return Ok(None);
        };

        Ok(self
            .resolve_text_field(fields, spec)?
            .and_then(|value| self.parse_date(&value, formats)))
    }

    /// 映射数值字段，并应用可选 transform。
    fn map_decimal(
        &self,
        fields: &HashMap<String, String>,
        spec: Option<&FieldSpec>,
    ) -> ImporterResult<Option<Decimal>> {
        let Some(spec) = spec else {
            return Ok(None);
        };

        Ok(self
            .resolve_text_field(fields, spec)?
            .and_then(|value| parse_decimal_with_transform(&value, spec.transformer())))
    }

    /// 映射文本字段。
    fn map_text(
        &self,
        fields: &HashMap<String, String>,
        spec: Option<&FieldSpec>,
    ) -> ImporterResult<Option<String>> {
        let Some(spec) = spec else {
            return Ok(None);
        };

        self.resolve_text_field(fields, spec)
    }

    fn map_extra_fields(
        &self,
        fields: &HashMap<String, String>,
        mapping: &FieldMapping,
        record: &mut RawRecord,
    ) {
        // 推荐写法：extra_key -> csv_column。
        // 兼容旧写法：csv_column -> extra_key。
        for (left, right) in &mapping.extra_fields {
            if let Some(value) = self.non_empty_value(fields.get(right).map(String::as_str)) {
                record.extra.insert(left.clone(), value.to_string());
            } else if let Some(value) = self.non_empty_value(fields.get(left).map(String::as_str)) {
                record.extra.insert(right.clone(), value.to_string());
            }
        }
    }

    /// 解析一个文本字段，支持默认值和 regex_extract。
    fn resolve_text_field(
        &self,
        fields: &HashMap<String, String>,
        spec: &FieldSpec,
    ) -> ImporterResult<Option<String>> {
        let base_value = fields
            .get(spec.column_name())
            .and_then(|value| self.non_empty_value(Some(value.as_str())))
            .or_else(|| {
                spec.default_value()
                    .and_then(|value| self.non_empty_value(Some(value)))
            });

        let Some(base_value) = base_value else {
            return Ok(None);
        };

        self.apply_regex_extract(spec, base_value)
    }

    /// 若配置了 `regex_extract`，则按正则提取字段值。
    fn apply_regex_extract(&self, spec: &FieldSpec, value: &str) -> ImporterResult<Option<String>> {
        let Some(pattern) = spec.regex_extract_pattern() else {
            return Ok(Some(value.to_string()));
        };

        let regex = Regex::new(pattern).map_err(|error| {
            ImporterError::Config(format!(
                "Invalid regex_extract '{}' for column '{}': {}",
                pattern,
                spec.column_name(),
                error
            ))
        })?;

        let captures = match regex.captures(value) {
            Some(captures) => captures,
            None => return Ok(None),
        };

        let matched = captures
            .get(1)
            .or_else(|| captures.get(0))
            .map(|value| value.as_str())
            .and_then(|value| self.non_empty_value(Some(value)))
            .map(str::to_string);

        Ok(matched)
    }

    /// 把空字符串或全空白字符串转换为 `None`。
    fn non_empty_value<'a>(&self, value: Option<&'a str>) -> Option<&'a str> {
        value.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
    }

    /// 先按日期时间解析，再按日期解析。
    fn parse_date(&self, value: &str, formats: &[String]) -> Option<NaiveDate> {
        let trimmed = value.trim();

        for format in formats {
            if let Ok(date_time) = NaiveDateTime::parse_from_str(trimmed, format) {
                return Some(date_time.date());
            }

            if let Ok(date) = NaiveDate::parse_from_str(trimmed, format) {
                return Some(date);
            }
        }

        const COMMON_DATE_FORMATS: [&str; 7] = [
            "%Y%m%d",
            "%Y-%m-%d",
            "%Y/%m/%d",
            "%Y.%m.%d",
            "%Y-%m-%d %H:%M:%S",
            "%Y/%m/%d %H:%M:%S",
            "%Y-%m-%d %H:%M",
        ];
        for format in COMMON_DATE_FORMATS {
            if let Ok(date_time) = NaiveDateTime::parse_from_str(trimmed, format) {
                return Some(date_time.date());
            }

            if let Ok(date) = NaiveDate::parse_from_str(trimmed, format) {
                return Some(date);
            }
        }

        parse_excel_serial_date(trimmed)
    }
}

pub(super) fn normalize_cell_value(value: &str) -> String {
    let trimmed = value.trim();
    strip_excel_quoted_literal(trimmed).unwrap_or_else(|| trimmed.to_string())
}

fn strip_excel_quoted_literal(value: &str) -> Option<String> {
    // Excel 导出中常见格式：="0.00" / ="240599141221"。
    // 这里仅做保守展开：必须是 `=` + 双引号字面量。
    if !value.starts_with('=') {
        return None;
    }

    let expression = value[1..].trim();
    if expression.len() < 2 || !expression.starts_with('"') || !expression.ends_with('"') {
        return None;
    }

    let inner = &expression[1..expression.len() - 1];
    Some(inner.replace("\"\"", "\""))
}
