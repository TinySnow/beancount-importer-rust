//! 字段解析工具。
//!
//! 本模块提供从字段映射中解析日期、数值、文本的通用方法，
//! 供 record_mapper 和 fallback 逻辑复用。

use std::collections::HashMap;

use chrono::{NaiveDate, NaiveDateTime};
use regex::Regex;
use rust_decimal::Decimal;

use crate::{
    error::{ImporterError, ImporterResult},
    model::mapping::field_spec::FieldSpec,
    utils::{decimal::parse_decimal_with_transform, time::parse_excel_serial_date},
};

use crate::runtime::reader::tabular::TabularRecordReader;

impl TabularRecordReader {
    /// 从候选键中取第一个非空文本值。
    pub(super) fn first_non_empty_text(
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
    pub(super) fn first_non_empty_decimal(
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
    pub(super) fn map_date(
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
    pub(super) fn map_decimal(
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
    pub(super) fn map_text(
        &self,
        fields: &HashMap<String, String>,
        spec: Option<&FieldSpec>,
    ) -> ImporterResult<Option<String>> {
        let Some(spec) = spec else {
            return Ok(None);
        };

        self.resolve_text_field(fields, spec)
    }

    /// 解析一个文本字段，支持默认值和 regex_extract。
    pub(super) fn resolve_text_field(
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
    pub(super) fn non_empty_value<'a>(&self, value: Option<&'a str>) -> Option<&'a str> {
        value.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
    }

    /// 解析日期文本。
    ///
    /// 解析顺序：
    /// 1. 用户配置格式；
    /// 2. 内置常见格式；
    /// 3. Excel 序列日期。
    pub(super) fn parse_date(&self, value: &str, formats: &[String]) -> Option<NaiveDate> {
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
