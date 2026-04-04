//! 模块说明：CSV/XLS 源读取与字段映射解析能力。
//!
//! 文件路径：src/model/reader/tabular/mapping/tests.rs。
//! 该文件主要包含单元测试与回归测试。
//! 关键符号：strict_mode_fails_on_field_count_mismatch、strict_mode_fails_on_mapping_error、non_strict_mode_skips_mapping_error、normalizes_excel_equals_quoted_literals。

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::model::{
    config::tabular_options::TabularOptions,
    mapping::{
        field_mapping::FieldMapping,
        field_spec::{DetailedFieldSpec, FieldSpec},
    },
};

use super::mapper::normalize_cell_value;
use crate::model::reader::tabular::{
    TabularRecordReader,
    table::{RowData, TabularData},
};

#[test]
fn strict_mode_fails_on_field_count_mismatch() {
    let reader = TabularRecordReader::new(TabularOptions::default(), 0, true, true);
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
    let reader = TabularRecordReader::new(TabularOptions::default(), 0, true, true);

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
    let reader = TabularRecordReader::new(TabularOptions::default(), 0, true, false);

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
    let reader = TabularRecordReader::new(TabularOptions::default(), 0, true, false);

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

#[test]
fn parses_excel_serial_date_for_date_field() {
    let reader = TabularRecordReader::new(TabularOptions::default(), 0, true, false);

    let mapping = FieldMapping {
        date: Some(FieldSpec::Simple("交易时间".to_string())),
        amount: Some(FieldSpec::Simple("金额(元)".to_string())),
        ..FieldMapping::default()
    };

    let table = TabularData {
        source_name: "XLSX",
        headers: vec!["交易时间".to_string(), "金额(元)".to_string()],
        rows: vec![RowData {
            line_no: 19,
            cells: vec!["46110.56767361111".to_string(), "0.03".to_string()],
        }],
        pre_parse_errors: 0,
    };

    let records = reader
        .map_table_to_records(table, Some(&mapping))
        .expect("mapping should succeed");
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].date,
        NaiveDate::from_ymd_opt(2026, 3, 29),
        "excel serial date should map to expected calendar date"
    );
}

#[test]
fn infers_date_amount_direction_and_payee_from_ccb_split_columns() {
    let reader = TabularRecordReader::new(TabularOptions::default(), 0, true, false);

    // 模拟旧映射列名与新网银导出列名不一致的场景。
    let mapping = FieldMapping {
        date: Some(FieldSpec::Simple("交易时间".to_string())),
        amount: Some(FieldSpec::Simple("交易金额".to_string())),
        payee: Some(FieldSpec::Simple("交易对方".to_string())),
        transaction_type: Some(FieldSpec::Simple("收/支".to_string())),
        narration: Some(FieldSpec::Simple("摘要".to_string())),
        date_formats: vec!["%Y-%m-%d".to_string()],
        ..FieldMapping::default()
    };

    let table = TabularData {
        source_name: "XLSX",
        headers: vec![
            "记账日".to_string(),
            "交易日期".to_string(),
            "交易时间".to_string(),
            "支出".to_string(),
            "收入".to_string(),
            "币种".to_string(),
            "摘要".to_string(),
            "对方户名".to_string(),
        ],
        rows: vec![
            RowData {
                line_no: 10,
                cells: vec![
                    "2026-03-29".to_string(),
                    "2026-03-29".to_string(),
                    "10:21:03".to_string(),
                    "35.20".to_string(),
                    "".to_string(),
                    "CNY".to_string(),
                    "餐饮消费".to_string(),
                    "某商户".to_string(),
                ],
            },
            RowData {
                line_no: 11,
                cells: vec![
                    "2026-03-30".to_string(),
                    "2026-03-30".to_string(),
                    "08:00:00".to_string(),
                    "".to_string(),
                    "100.00".to_string(),
                    "CNY".to_string(),
                    "工资入账".to_string(),
                    "公司账户".to_string(),
                ],
            },
        ],
        pre_parse_errors: 0,
    };

    let records = reader
        .map_table_to_records(table, Some(&mapping))
        .expect("mapping should succeed");
    assert_eq!(records.len(), 2);

    assert_eq!(records[0].date, NaiveDate::from_ymd_opt(2026, 3, 29));
    assert_eq!(records[0].amount, Some(dec!(35.20)));
    assert_eq!(records[0].transaction_type.as_deref(), Some("支出"));
    assert_eq!(records[0].payee.as_deref(), Some("某商户"));
    assert_eq!(records[0].currency.as_deref(), Some("CNY"));

    assert_eq!(records[1].date, NaiveDate::from_ymd_opt(2026, 3, 30));
    assert_eq!(records[1].amount, Some(dec!(100.00)));
    assert_eq!(records[1].transaction_type.as_deref(), Some("收入"));
    assert_eq!(records[1].payee.as_deref(), Some("公司账户"));
}

#[test]
fn infers_amount_and_direction_from_icbc_ledger_split_columns() {
    let reader = TabularRecordReader::new(TabularOptions::default(), 0, true, false);

    // 模拟工行“记账金额(收入/支出)”拆列，无独立收支标识列。
    let mapping = FieldMapping {
        date: Some(FieldSpec::Simple("交易日期".to_string())),
        narration: Some(FieldSpec::Simple("摘要".to_string())),
        date_formats: vec!["%Y-%m-%d".to_string()],
        ..FieldMapping::default()
    };

    let table = TabularData {
        source_name: "CSV",
        headers: vec![
            "交易日期".to_string(),
            "摘要".to_string(),
            "记账金额(收入)".to_string(),
            "记账金额(支出)".to_string(),
            "记账币种".to_string(),
            "对方户名".to_string(),
        ],
        rows: vec![
            RowData {
                line_no: 2,
                cells: vec![
                    "2026-03-29".to_string(),
                    "财付通转账".to_string(),
                    "2.77".to_string(),
                    "".to_string(),
                    "人民币".to_string(),
                    "财付通支付科技有限公司".to_string(),
                ],
            },
            RowData {
                line_no: 3,
                cells: vec![
                    "2026-03-29".to_string(),
                    "消费".to_string(),
                    "".to_string(),
                    "2,010.00".to_string(),
                    "人民币".to_string(),
                    "支付宝（中国）网络技术有限公司".to_string(),
                ],
            },
        ],
        pre_parse_errors: 0,
    };

    let records = reader
        .map_table_to_records(table, Some(&mapping))
        .expect("mapping should succeed");
    assert_eq!(records.len(), 2);

    assert_eq!(records[0].amount, Some(dec!(2.77)));
    assert_eq!(records[0].transaction_type.as_deref(), Some("收入"));
    assert_eq!(
        records[0].extra.get("type").map(String::as_str),
        Some("收入")
    );

    assert_eq!(records[1].amount, Some(dec!(2010.00)));
    assert_eq!(records[1].transaction_type.as_deref(), Some("支出"));
    assert_eq!(
        records[1].extra.get("type").map(String::as_str),
        Some("支出")
    );
}

#[test]
fn keeps_explicit_type_extra_without_overwrite() {
    let reader = TabularRecordReader::new(TabularOptions::default(), 0, true, false);

    let mut mapping = FieldMapping {
        date: Some(FieldSpec::Simple("交易日期".to_string())),
        narration: Some(FieldSpec::Simple("摘要".to_string())),
        date_formats: vec!["%Y-%m-%d".to_string()],
        ..FieldMapping::default()
    };
    mapping
        .extra_fields
        .insert("type".to_string(), "业务方向".to_string());

    let table = TabularData {
        source_name: "CSV",
        headers: vec![
            "交易日期".to_string(),
            "摘要".to_string(),
            "业务方向".to_string(),
            "记账金额(收入)".to_string(),
            "记账金额(支出)".to_string(),
        ],
        rows: vec![RowData {
            line_no: 2,
            cells: vec![
                "2026-03-29".to_string(),
                "财付通转账".to_string(),
                "手工方向".to_string(),
                "2.77".to_string(),
                "".to_string(),
            ],
        }],
        pre_parse_errors: 0,
    };

    let records = reader
        .map_table_to_records(table, Some(&mapping))
        .expect("mapping should succeed");
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].extra.get("type").map(String::as_str),
        Some("手工方向")
    );
}

#[test]
fn normalizes_pay_time_from_excel_serial_extra_field() {
    let reader = TabularRecordReader::new(TabularOptions::default(), 0, true, false);

    let mut mapping = FieldMapping {
        date: Some(FieldSpec::Simple("交易时间".to_string())),
        amount: Some(FieldSpec::Simple("金额".to_string())),
        date_formats: vec!["%Y-%m-%d %H:%M:%S".to_string()],
        ..FieldMapping::default()
    };
    mapping
        .extra_fields
        .insert("payTime".to_string(), "交易时间".to_string());

    let table = TabularData {
        source_name: "XLSX",
        headers: vec!["交易时间".to_string(), "金额".to_string()],
        rows: vec![RowData {
            line_no: 2,
            cells: vec!["46110.5".to_string(), "10".to_string()],
        }],
        pre_parse_errors: 0,
    };

    let records = reader
        .map_table_to_records(table, Some(&mapping))
        .expect("mapping should succeed");
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].extra.get("payTime").map(String::as_str),
        Some("12:00:00")
    );
}

#[test]
fn normalizes_pay_time_from_datetime_text_extra_field() {
    let reader = TabularRecordReader::new(TabularOptions::default(), 0, true, false);

    let mut mapping = FieldMapping {
        date: Some(FieldSpec::Simple("交易时间".to_string())),
        amount: Some(FieldSpec::Simple("金额".to_string())),
        date_formats: vec!["%Y-%m-%d %H:%M:%S".to_string()],
        ..FieldMapping::default()
    };
    mapping
        .extra_fields
        .insert("payTime".to_string(), "交易时间".to_string());

    let table = TabularData {
        source_name: "CSV",
        headers: vec!["交易时间".to_string(), "金额".to_string()],
        rows: vec![RowData {
            line_no: 2,
            cells: vec!["2026-03-06 14:37:15".to_string(), "10".to_string()],
        }],
        pre_parse_errors: 0,
    };

    let records = reader
        .map_table_to_records(table, Some(&mapping))
        .expect("mapping should succeed");
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].extra.get("payTime").map(String::as_str),
        Some("14:37:15")
    );
}
