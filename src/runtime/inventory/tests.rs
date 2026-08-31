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
    // cutoff 取当前批次卖出日期（2026-01-06），seed 买入（2025-12-26）早于截止点，仍应被回放。
    let cutoff = NaiveDate::from_ymd_opt(2026, 1, 6);
    let mut inventory = load_seed_inventory_from_files(&seed_files, cutoff);
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

#[test]
fn skips_seed_transactions_at_or_after_cutoff() {
    let mut seed_path = std::env::temp_dir();
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    seed_path.push(format!(
        "beancount-seed-cutoff-{}-{}.bean",
        std::process::id(),
        unique
    ));

    // 两笔交易：一笔早于截止点，一笔晚于截止点。
    let seed_content = r#"
2026-01-05 * "past buy" "past buy"
  Assets:Invest:Broker:Securities  100 SEC_161226 {1.5 CNY}
  Assets:Invest:Broker:Cash  -150 CNY

2026-01-07 * "future buy" "future buy"
  Assets:Invest:Broker:Securities  200 SEC_161226 {2.0 CNY}
  Assets:Invest:Broker:Cash  -400 CNY
"#;
    fs::write(&seed_path, seed_content).expect("seed file should be writable");

    let cutoff = NaiveDate::from_ymd_opt(2026, 1, 6);
    let seed_files = vec![seed_path.to_string_lossy().to_string()];
    let inventory = load_seed_inventory_from_files(&seed_files, cutoff);

    // 只有早于截止点的 lot 被回放，达到或超过截止点的交易被跳过。
    let key = (
        "Assets:Invest:Broker:Securities".to_string(),
        "SEC_161226".to_string(),
    );
    let lots = &inventory.lots[&key];
    assert_eq!(lots.len(), 1);
    assert_eq!(lots[0].remaining, dec!(100));
    assert_eq!(lots[0].cost.number, dec!(1.5));

    let _ = fs::remove_file(seed_path);
}

#[test]
fn assigns_split_new_share_cost_from_removed_lots() {
    let buy = Transaction::new(
        NaiveDate::from_ymd_opt(2026, 1, 14).expect("valid date"),
        "buy before split",
    )
    .with_posting(
        Posting::new("Assets:Invest:Broker:Securities")
            .with_amount(Amount::new(dec!(200), "SEC_159516"))
            .with_cost(Cost::new(dec!(1.771), "CNY")),
    );

    let split = Transaction::new(
        NaiveDate::from_ymd_opt(2026, 1, 15).expect("valid date"),
        "ETF share split",
    )
    .with_posting(
        Posting::new("Assets:Invest:Broker:Securities")
            .with_amount(Amount::new(dec!(-200), "SEC_159516"))
            .with_inferred_cost(),
    )
    .with_posting(
        Posting::new("Assets:Invest:Broker:Securities")
            .with_amount(Amount::new(dec!(400), "SEC_159516")),
    );

    let mut transactions = vec![buy, split];
    resolve_inferred_cost_postings(&mut transactions);

    let split_tx = &transactions[1];
    let postings = split_tx
        .postings
        .iter()
        .filter(|posting| posting.account == "Assets:Invest:Broker:Securities")
        .collect::<Vec<_>>();

    // 移除 200 + 新增 400 共两条腿
    assert_eq!(postings.len(), 2);

    // 新份额 400 股，成本 = 200 * 1.771 / 400 = 0.8855
    let new_posting = postings
        .iter()
        .find(|posting| {
            posting
                .amount
                .as_ref()
                .map(|amount| amount.number.is_sign_positive())
                .unwrap_or(false)
        })
        .expect("split should have a positive posting");
    assert_eq!(
        new_posting.amount.as_ref().map(|amount| amount.number),
        Some(dec!(400))
    );
    assert_eq!(
        new_posting.cost.as_ref().map(|cost| cost.number),
        Some(dec!(0.8855))
    );
    assert_eq!(
        new_posting.cost.as_ref().map(|cost| cost.currency.as_str()),
        Some("CNY")
    );
}
