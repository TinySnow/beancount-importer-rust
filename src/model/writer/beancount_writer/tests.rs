//! 模块说明：Beancount 渲染输出实现。
//!
//! 文件路径：src/model/writer/beancount_writer/tests.rs。
//! 该文件主要包含单元测试与回归测试。
//! 关键符号：test_simple_transaction、test_quoted_date_format_is_sanitized、test_inferred_cost_posting_is_rendered_as_empty_braces、test_open_directives_are_emitted_when_enabled。

use chrono::NaiveDate;
use rust_decimal_macros::dec;

use super::*;
use crate::model::account::{amount::Amount, cost::Cost, posting::Posting, price::Price};

#[test]
fn test_simple_transaction() {
    let tx = Transaction::new(
        NaiveDate::from_ymd_opt(2024, 1, 15).expect("valid date"),
        "Coffee at Starbucks",
    )
    .with_payee("Starbucks")
    .with_posting(Posting::new("Expenses:Food:Coffee").with_amount(Amount::new(dec!(35.00), "CNY")))
    .with_posting(Posting::new("Assets:Cash"));

    let writer = BeancountWriter::new(OutputConfig::default());
    let mut output = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut output);
        writer
            .write(&[tx], &mut cursor)
            .expect("writer should succeed");
    }

    let result = String::from_utf8(output).expect("utf8 output");
    assert!(result.contains("2024-01-15 * \"Starbucks\" \"Coffee at Starbucks\""));
    assert!(result.contains("Expenses:Food:Coffee  35.00 CNY"));
    assert!(result.contains("Assets:Cash"));
}

#[test]
fn test_quoted_date_format_is_sanitized() {
    let tx = Transaction::new(
        NaiveDate::from_ymd_opt(2024, 3, 1).expect("valid date"),
        "Quoted date format",
    )
    .with_posting(Posting::new("Assets:Cash"));

    let config = OutputConfig {
        date_format: "\"%Y-%m-%d\"".to_string(),
        ..OutputConfig::default()
    };
    let writer = BeancountWriter::new(config);

    let mut output = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut output);
        writer
            .write(&[tx], &mut cursor)
            .expect("writer should succeed");
    }

    let result = String::from_utf8(output).expect("utf8 output");
    assert!(result.starts_with("2024-03-01 *"));
    assert!(!result.starts_with("\"2024-03-01\" *"));
}

#[test]
fn test_inferred_cost_posting_is_rendered_as_empty_braces() {
    let tx = Transaction::new(
        NaiveDate::from_ymd_opt(2024, 3, 2).expect("valid date"),
        "Sell security",
    )
    .with_posting(
        Posting::new("Assets:Broker:Securities")
            .with_amount(Amount::new(dec!(-10), "SEC_123456"))
            .with_inferred_cost()
            .with_price(Price::new(dec!(1.23), "CNY")),
    )
    .with_posting(Posting::new("Assets:Broker:Cash").with_amount(Amount::new(dec!(12.30), "CNY")))
    .with_posting(Posting::new("Income:Investing:Capital-Gains"));

    let writer = BeancountWriter::new(OutputConfig::default());
    let mut output = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut output);
        writer
            .write(&[tx], &mut cursor)
            .expect("writer should succeed");
    }

    let result = String::from_utf8(output).expect("utf8 output");
    assert!(result.contains("-10.00 SEC_123456 {} @ 1.23 CNY"));
}

#[test]
fn test_open_directives_are_emitted_when_enabled() {
    let tx = Transaction::new(
        NaiveDate::from_ymd_opt(2024, 5, 1).expect("valid date"),
        "Buy fund",
    )
    .with_posting(
        Posting::new("Assets:Broker:Securities")
            .with_amount(Amount::new(dec!(10), "SEC_123456"))
            .with_cost(Cost::new(dec!(1.23), "CNY")),
    )
    .with_posting(Posting::new("Assets:Broker:Cash").with_amount(Amount::new(dec!(-12.30), "CNY")))
    .with_posting(
        Posting::new("Expenses:Investing:Fees").with_amount(Amount::new(dec!(0.10), "CNY")),
    );

    let config = OutputConfig {
        emit_open_directives: true,
        open_date: Some("2024-01-01".to_string()),
        ..OutputConfig::default()
    };
    let writer = BeancountWriter::new(config);

    let mut output = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut output);
        writer
            .write(&[tx], &mut cursor)
            .expect("writer should succeed");
    }

    let result = String::from_utf8(output).expect("utf8 output");
    assert!(result.contains("2024-01-01 open Assets:Broker:Cash CNY"));
    assert!(result.contains("2024-01-01 open Assets:Broker:Securities"));
    assert!(result.contains("2024-01-01 open Expenses:Investing:Fees CNY"));
}

#[test]
fn test_open_directives_include_booking_method_for_non_fiat_accounts() {
    let tx = Transaction::new(
        NaiveDate::from_ymd_opt(2024, 5, 1).expect("valid date"),
        "Buy fund",
    )
    .with_posting(
        Posting::new("Assets:Broker:Securities")
            .with_amount(Amount::new(dec!(10), "SEC_123456"))
            .with_cost(Cost::new(dec!(1.23), "CNY")),
    )
    .with_posting(Posting::new("Assets:Broker:Cash").with_amount(Amount::new(dec!(-12.30), "CNY")));

    let config = OutputConfig {
        emit_open_directives: true,
        open_date: Some("2024-01-01".to_string()),
        booking_method: Some("fifo".to_string()),
        ..OutputConfig::default()
    };
    let writer = BeancountWriter::new(config);

    let mut output = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut output);
        writer
            .write(&[tx], &mut cursor)
            .expect("writer should succeed");
    }

    let result = String::from_utf8(output).expect("utf8 output");
    assert!(result.contains("2024-01-01 open Assets:Broker:Securities \"FIFO\""));
    assert!(result.contains("2024-01-01 open Assets:Broker:Cash CNY"));
}

#[test]
fn test_commodity_directive_uses_date_and_lowercase_keyword() {
    let tx = Transaction::new(
        NaiveDate::from_ymd_opt(2024, 5, 1).expect("valid date"),
        "Buy fund",
    )
    .with_posting(
        Posting::new("Assets:Broker:Securities")
            .with_amount(Amount::new(dec!(10), "SEC_123456"))
            .with_cost(Cost::new(dec!(1.23), "CNY")),
    )
    .with_posting(Posting::new("Assets:Broker:Cash").with_amount(Amount::new(dec!(-12.30), "CNY")));

    let writer = BeancountWriter::new(OutputConfig::default());
    let mut output = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut output);
        writer
            .write(&[tx], &mut cursor)
            .expect("writer should succeed");
    }

    let result = String::from_utf8(output).expect("utf8 output");
    assert!(result.contains("2024-05-01 commodity SEC_123456"));
    assert!(!result.contains("COMMODITY"));
}

#[test]
fn test_tags_and_links_are_emitted_on_header_line() {
    let tx = Transaction::new(
        NaiveDate::from_ymd_opt(2024, 6, 1).expect("valid date"),
        "Tagged transaction",
    )
    .with_payee("Payee")
    .with_tag("food")
    .with_tag("lunch")
    .with_link("order123")
    .with_posting(Posting::new("Expenses:Food").with_amount(Amount::new(dec!(10), "CNY")))
    .with_posting(Posting::new("Assets:Cash").with_amount(Amount::new(dec!(-10), "CNY")));

    let writer = BeancountWriter::new(OutputConfig::default());
    let mut output = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut output);
        writer
            .write(&[tx], &mut cursor)
            .expect("writer should succeed");
    }

    let result = String::from_utf8(output).expect("utf8 output");
    assert!(result.contains("\"Payee\" \"Tagged transaction\" #food #lunch ^order123"));
    assert!(!result.contains("; Tags:"));
    assert!(!result.contains("; Links:"));
}
