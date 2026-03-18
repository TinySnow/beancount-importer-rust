//! 模块说明：CSV/XLS 源读取与字段映射解析能力。
//!
//! 文件路径：src/model/reader/csv_reader/mapper/tests.rs。
//! 该文件主要包含单元测试与回归测试。
//! 关键符号：strict_mode_fails_on_field_count_mismatch、strict_mode_fails_on_mapping_error、non_strict_mode_skips_mapping_error、normalizes_excel_equals_quoted_literals。

use rust_decimal::Decimal;

use crate::model::{
    config::csv_options::CsvOptions,
    mapping::{
        field_mapping::FieldMapping,
        field_spec::{DetailedFieldSpec, FieldSpec},
    },
};

use super::super::{
    CsvRecordReader,
    table::{RowData, TabularData},
};
use super::normalize_cell_value;

#[test]
fn strict_mode_fails_on_field_count_mismatch() {
    let reader = CsvRecordReader::new(CsvOptions::default(), 0, true, true);
    let table = TabularData {
        source_name: "CSV",
        headers: vec!["A".to_string(), "B".to_string()],
        rows: vec![RowData {
            line_no: 2,
            cells: vec!["value".to_string()],
        }],
        pre_parse_errors: 0,
    };

    let result = reader.map_table_to_records(table, None);
    assert!(
        result.is_err(),
        "strict mode should fail on field count mismatch"
    );
}

#[test]
fn strict_mode_fails_on_mapping_error() {
    let reader = CsvRecordReader::new(CsvOptions::default(), 0, true, true);

    let mapping = FieldMapping {
        payee: Some(FieldSpec::Detailed(DetailedFieldSpec {
            column: "A".to_string(),
            default: None,
            transform: None,
            regex_extract: Some("(".to_string()),
        })),
        ..FieldMapping::default()
    };

    let table = TabularData {
        source_name: "CSV",
        headers: vec!["A".to_string()],
        rows: vec![RowData {
            line_no: 2,
            cells: vec!["value".to_string()],
        }],
        pre_parse_errors: 0,
    };

    let result = reader.map_table_to_records(table, Some(&mapping));
    assert!(result.is_err(), "strict mode should fail on mapping error");
}

#[test]
fn non_strict_mode_skips_mapping_error() {
    let reader = CsvRecordReader::new(CsvOptions::default(), 0, true, false);

    let mapping = FieldMapping {
        payee: Some(FieldSpec::Detailed(DetailedFieldSpec {
            column: "A".to_string(),
            default: None,
            transform: None,
            regex_extract: Some("(".to_string()),
        })),
        ..FieldMapping::default()
    };

    let table = TabularData {
        source_name: "CSV",
        headers: vec!["A".to_string()],
        rows: vec![RowData {
            line_no: 2,
            cells: vec!["value".to_string()],
        }],
        pre_parse_errors: 0,
    };

    let result = reader
        .map_table_to_records(table, Some(&mapping))
        .expect("non-strict mode should keep going");
    assert!(result.is_empty());
}

#[test]
fn normalizes_excel_equals_quoted_literals() {
    assert_eq!(normalize_cell_value("=\"0\""), "0");
    assert_eq!(normalize_cell_value("=\"0.00\""), "0.00");
    assert_eq!(normalize_cell_value("=\"240599141221\""), "240599141221");
    assert_eq!(normalize_cell_value("  =\"abc\"  "), "abc");
    assert_eq!(normalize_cell_value("=SUM(A1:A3)"), "=SUM(A1:A3)");
}

#[test]
fn maps_amount_and_extra_fields_after_excel_literal_normalization() {
    let reader = CsvRecordReader::new(CsvOptions::default(), 0, true, false);

    let mut mapping = FieldMapping {
        amount: Some(FieldSpec::Simple("amount".to_string())),
        ..FieldMapping::default()
    };
    mapping
        .extra_fields
        .insert("productAccount".to_string(), "product".to_string());

    let table = TabularData {
        source_name: "CSV",
        headers: vec!["amount".to_string(), "product".to_string()],
        rows: vec![RowData {
            line_no: 2,
            cells: vec!["=\"0.00\"".to_string(), "=\"240599141221\"".to_string()],
        }],
        pre_parse_errors: 0,
    };

    let records = reader
        .map_table_to_records(table, Some(&mapping))
        .expect("mapping should succeed");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].amount, Some(Decimal::new(0, 2)));
    assert_eq!(
        records[0].extra.get("productAccount").map(String::as_str),
        Some("240599141221")
    );
}
