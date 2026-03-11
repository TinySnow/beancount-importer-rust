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
    utils::decimal::parse_decimal_with_transform,
};

use super::{CsvRecordReader, table::TabularData};

impl CsvRecordReader {
    /// 将统一表格结构映射为 `RawRecord`。
    pub(super) fn map_table_to_records(
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

        for row in table.rows {
            if row.cells.len() != expected_columns {
                warn!(
                    "Line {}: field count mismatch (expected {}, got {})",
                    row.line_no,
                    expected_columns,
                    row.cells.len()
                );
            }

            let field_map = table
                .headers
                .iter()
                .zip(row.cells.iter())
                .map(|(header, value)| (header.clone(), value.trim().to_string()))
                .collect::<HashMap<_, _>>();

            match self.map_to_raw_record(&field_map, mapping) {
                Ok(record) => records.push(record),
                Err(error) => {
                    mapping_errors += 1;
                    warn!("Line {}: mapping error - {}", row.line_no, error);
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

    /// 校验映射中声明的列名是否存在于表头。
    fn validate_mapping(&self, mapping: &FieldMapping, headers: &[String]) {
        for (name, spec) in Self::mapped_specs(mapping) {
            if let Some(spec) = spec {
                let column = spec.column_name();
                if headers.iter().any(|header| header == column) {
                    trace!("Mapping '{}' -> '{}'", name, column);
                } else {
                    warn!(
                        "Mapping field '{}' references column '{}' that is not in CSV headers",
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

        Ok(record)
    }

    /// 将指定列映射为日期字段。
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

    /// 将指定列映射为十进制数值，并应用可选转换（如 `abs`）。
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

    /// 将指定列映射为文本字段。
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
        // 推荐格式：`extra_key -> csv_column`
        // 仍兼容旧格式：`csv_column -> extra_key`
        for (left, right) in &mapping.extra_fields {
            if let Some(value) = self.non_empty_value(fields.get(right).map(String::as_str)) {
                record.extra.insert(left.clone(), value.to_string());
            } else if let Some(value) = self.non_empty_value(fields.get(left).map(String::as_str)) {
                record.extra.insert(right.clone(), value.to_string());
            }
        }
    }

    /// 解析并提取一个文本字段，支持默认值与正则提取。
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

    /// 对字段值应用 `regex_extract`。
    ///
    /// 规则：
    /// - 有捕获组时优先取第一个捕获组；
    /// - 无捕获组时取整体匹配；
    /// - 未匹配时返回空值（而非报错）。
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

    /// 将空串或纯空白字符串归一化为空值。
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

    /// 按配置的日期格式列表顺序尝试解析。
    ///
    /// 同时支持日期时间格式和纯日期格式。
    fn parse_date(&self, value: &str, formats: &[String]) -> Option<NaiveDate> {
        for format in formats {
            if let Ok(date_time) = NaiveDateTime::parse_from_str(value, format) {
                return Some(date_time.date());
            }

            if let Ok(date) = NaiveDate::parse_from_str(value, format) {
                return Some(date);
            }
        }

        None
    }
}
