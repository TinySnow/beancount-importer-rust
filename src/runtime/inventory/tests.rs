//! 模块说明：证券库存 lot 匹配、种子加载与成本补全能力。
//!
//! 文件路径：src/runtime/inventory/tests.rs。
//! 该文件主要包含单元测试与回归测试。
//! 关键符号：resolves_inferred_sell_into_explicit_fifo_lots、keeps_residual_inferred_posting_when_lots_are_insufficient、resolves_explicit_cost_sell_without_date_into_dated_fifo_lots、keeps_residual_explicit_cost_sell_when_lots_are_insufficient。

use std::fs;

use chrono::NaiveDate;
use rust_decimal_macros::dec;

use crate::model::{
    account::{amount::Amount, cost::Cost, posting::Posting, price::Price},
    transaction::Transaction,
};

use super::{
    load_seed_inventory_from_files, resolve_inferred_cost_postings,
    resolve_inferred_cost_postings_with_inventory,
};

#[test]
fn resolves_inferred_sell_into_explicit_fifo_lots() {
    let buy_1 = Transaction::new(
        NaiveDate::from_ymd_opt(2025, 12, 23).expect("valid date"),
        "buy lot 1",
    )
    .with_posting(
        Posting::new("Assets:Invest:Broker:Securities")
            .with_amount(Amount::new(dec!(275), "SEC_161226"))
            .with_cost(Cost::new(dec!(1.7987), "CNY")),
    );

    let buy_2 = Transaction::new(
        NaiveDate::from_ymd_opt(2025, 12, 24).expect("valid date"),
        "buy lot 2",
    )
    .with_posting(
        Posting::new("Assets:Invest:Broker:Securities")
            .with_amount(Amount::new(dec!(267), "SEC_161226"))
            .with_cost(Cost::new(dec!(1.8527), "CNY")),
    );

    let sell = Transaction::new(
        NaiveDate::from_ymd_opt(2025, 12, 26).expect("valid date"),
        "sell mixed lots",
    )
    .with_posting(
        Posting::new("Assets:Invest:Broker:Securities")
            .with_amount(Amount::new(dec!(-523), "SEC_161226"))
            .with_inferred_cost()
            .with_price(Price::new(dec!(2.524), "CNY")),
    );

    let mut transactions = vec![buy_1, buy_2, sell];
    resolve_inferred_cost_postings(&mut transactions);

    let sell_tx = &transactions[2];
    let sell_postings = sell_tx
        .postings
        .iter()
        .filter(|posting| posting.account == "Assets:Invest:Broker:Securities")
        .collect::<Vec<_>>();

    assert_eq!(sell_postings.len(), 2);
    assert_eq!(
        sell_postings[0].amount.as_ref().map(|amount| amount.number),
        Some(dec!(-275))
    );
    assert_eq!(
        sell_postings[1].amount.as_ref().map(|amount| amount.number),
        Some(dec!(-248))
    );
    assert!(!sell_postings[0].inferred_cost);
    assert!(!sell_postings[1].inferred_cost);
    assert_eq!(
        sell_postings[0].cost.as_ref().map(|cost| cost.number),
        Some(dec!(1.7987))
    );
    assert_eq!(
        sell_postings[1].cost.as_ref().map(|cost| cost.number),
        Some(dec!(1.8527))
    );
}

#[test]
fn keeps_residual_inferred_posting_when_lots_are_insufficient() {
    let buy = Transaction::new(
        NaiveDate::from_ymd_opt(2025, 12, 23).expect("valid date"),
        "buy",
    )
    .with_posting(
        Posting::new("Assets:Invest:Broker:Securities")
            .with_amount(Amount::new(dec!(100), "SEC_161226"))
            .with_cost(Cost::new(dec!(1.7987), "CNY")),
    );

    let sell = Transaction::new(
        NaiveDate::from_ymd_opt(2025, 12, 24).expect("valid date"),
        "sell more than current file lots",
    )
    .with_posting(
        Posting::new("Assets:Invest:Broker:Securities")
            .with_amount(Amount::new(dec!(-150), "SEC_161226"))
            .with_inferred_cost()
            .with_price(Price::new(dec!(2.1000), "CNY")),
    );

    let mut transactions = vec![buy, sell];
    resolve_inferred_cost_postings(&mut transactions);

    let sell_tx = &transactions[1];
    let sell_postings = sell_tx
        .postings
        .iter()
        .filter(|posting| posting.account == "Assets:Invest:Broker:Securities")
        .collect::<Vec<_>>();

    assert_eq!(sell_postings.len(), 2);
    assert_eq!(
        sell_postings[0].amount.as_ref().map(|amount| amount.number),
        Some(dec!(-100))
    );
    assert_eq!(
        sell_postings[1].amount.as_ref().map(|amount| amount.number),
        Some(dec!(-50))
    );
    assert!(!sell_postings[0].inferred_cost);
    assert!(sell_postings[1].inferred_cost);
}

#[test]
fn resolves_explicit_cost_sell_without_date_into_dated_fifo_lots() {
    let buy_1 = Transaction::new(
        NaiveDate::from_ymd_opt(2026, 1, 14).expect("valid date"),
        "repo buy 1",
    )
    .with_posting(
        Posting::new("Assets:Invest:Broker:Securities")
            .with_amount(Amount::new(dec!(100), "SEC_131810"))
            .with_cost(Cost::new(dec!(100), "CNY")),
    );

    let buy_2 = Transaction::new(
        NaiveDate::from_ymd_opt(2026, 1, 15).expect("valid date"),
        "repo buy 2",
    )
    .with_posting(
        Posting::new("Assets:Invest:Broker:Securities")
            .with_amount(Amount::new(dec!(100), "SEC_131810"))
            .with_cost(Cost::new(dec!(100), "CNY")),
    );

    let sell = Transaction::new(
        NaiveDate::from_ymd_opt(2026, 1, 16).expect("valid date"),
        "repo mature",
    )
    .with_posting(
        Posting::new("Assets:Invest:Broker:Securities")
            .with_amount(Amount::new(dec!(-150), "SEC_131810"))
            .with_cost(Cost::new(dec!(100), "CNY")),
    );

    let mut transactions = vec![buy_1, buy_2, sell];
    resolve_inferred_cost_postings(&mut transactions);

    let sell_tx = &transactions[2];
    let sell_postings = sell_tx
        .postings
        .iter()
        .filter(|posting| posting.account == "Assets:Invest:Broker:Securities")
        .collect::<Vec<_>>();

    assert_eq!(sell_postings.len(), 2);
    assert_eq!(
        sell_postings[0].amount.as_ref().map(|amount| amount.number),
        Some(dec!(-100))
    );
    assert_eq!(
        sell_postings[1].amount.as_ref().map(|amount| amount.number),
        Some(dec!(-50))
    );
    assert_eq!(
        sell_postings[0].cost.as_ref().and_then(|cost| cost.date),
        NaiveDate::from_ymd_opt(2026, 1, 14)
    );
    assert_eq!(
        sell_postings[1].cost.as_ref().and_then(|cost| cost.date),
        NaiveDate::from_ymd_opt(2026, 1, 15)
    );
}

#[test]
fn keeps_residual_explicit_cost_sell_when_lots_are_insufficient() {
    let buy = Transaction::new(
        NaiveDate::from_ymd_opt(2026, 1, 14).expect("valid date"),
        "repo buy",
    )
    .with_posting(
        Posting::new("Assets:Invest:Broker:Securities")
            .with_amount(Amount::new(dec!(100), "SEC_131810"))
            .with_cost(Cost::new(dec!(100), "CNY")),
    );

    let sell = Transaction::new(
        NaiveDate::from_ymd_opt(2026, 1, 16).expect("valid date"),
        "repo mature oversized",
    )
    .with_posting(
        Posting::new("Assets:Invest:Broker:Securities")
            .with_amount(Amount::new(dec!(-130), "SEC_131810"))
            .with_cost(Cost::new(dec!(100), "CNY")),
    );

    let mut transactions = vec![buy, sell];
    resolve_inferred_cost_postings(&mut transactions);

    let sell_tx = &transactions[1];
    let sell_postings = sell_tx
        .postings
        .iter()
        .filter(|posting| posting.account == "Assets:Invest:Broker:Securities")
        .collect::<Vec<_>>();

    assert_eq!(sell_postings.len(), 2);
    assert_eq!(
        sell_postings[0].amount.as_ref().map(|amount| amount.number),
        Some(dec!(-100))
    );
    assert_eq!(
        sell_postings[1].amount.as_ref().map(|amount| amount.number),
        Some(dec!(-30))
    );
    assert_eq!(
        sell_postings[0].cost.as_ref().and_then(|cost| cost.date),
        NaiveDate::from_ymd_opt(2026, 1, 14)
    );
    assert_eq!(
        sell_postings[1].cost.as_ref().and_then(|cost| cost.date),
        None
    );
}

#[test]
fn resolves_sell_with_cross_period_seed_inventory() {
    let mut seed_path = std::env::temp_dir();
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    seed_path.push(format!(
        "beancount-seed-{}-{}.bean",
        std::process::id(),
        unique
    ));

    let seed_content = r#"
2025-12-26 * "seed buy" "seed buy"
  Assets:Invest:Broker:Securities  154 SEC_161226 {1.9469 CNY}
  Assets:Invest:Broker:Cash  -299.8226 CNY
"#;
    fs::write(&seed_path, seed_content).expect("seed file should be writable");

    let mut transactions = vec![
        Transaction::new(
            NaiveDate::from_ymd_opt(2026, 1, 6).expect("valid date"),
            "cross period sell",
        )
        .with_posting(
            Posting::new("Assets:Invest:Broker:Securities")
                .with_amount(Amount::new(dec!(-100), "SEC_161226"))
                .with_inferred_cost()
                .with_price(Price::new(dec!(2.53), "CNY")),
        ),
    ];

    let seed_files = vec![seed_path.to_string_lossy().to_string()];
    let mut inventory = load_seed_inventory_from_files(&seed_files);
    resolve_inferred_cost_postings_with_inventory(&mut transactions, &mut inventory);

    let sell_postings = transactions[0]
        .postings
        .iter()
        .filter(|posting| posting.account == "Assets:Invest:Broker:Securities")
        .collect::<Vec<_>>();
    assert_eq!(sell_postings.len(), 1);
    assert!(!sell_postings[0].inferred_cost);
    assert_eq!(
        sell_postings[0].cost.as_ref().map(|cost| cost.number),
        Some(dec!(1.9469))
    );
    assert_eq!(
        sell_postings[0].cost.as_ref().and_then(|cost| cost.date),
        NaiveDate::from_ymd_opt(2025, 12, 26)
    );

    let _ = fs::remove_file(seed_path);
}
