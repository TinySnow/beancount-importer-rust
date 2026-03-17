//! Runtime import pipeline and post-processing utilities.
mod config_loader;

use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    path::Path,
    str::FromStr,
};

use anyhow::{Context, Result, anyhow};
use log::{debug, info, warn};
use once_cell::sync::Lazy;
use regex::Regex;
use rust_decimal::Decimal;

use crate::{
    model::{
        account::{cost::Cost, posting::Posting},
        cli::Cli,
        config::meta_value::MetaValue,
        registry::provider_registry::ProviderRegistry,
        rule::rule_engine::RuleEngine,
        transaction::Transaction,
        writer::beancount_writer::BeancountWriter,
    },
    runtime::config_loader::load,
};

/// Runs the end-to-end import flow:
/// load configs, parse records, transform records, and write Beancount output.
pub fn run(cli: Cli) -> Result<()> {
    info!("Starting beancount-importer");
    debug!("Provider: {}", cli.provider);
    debug!("Source file: {}", cli.source.display());
    debug!("Config file: {}", cli.config.display());

    let loaded = load(&cli)?;

    let registry = ProviderRegistry::global();
    let provider = registry.get(&cli.provider).with_context(|| {
        format!(
            "Unknown provider '{}'. Available providers: {:?}",
            cli.provider,
            registry.list_providers()
        )
    })?;

    info!(
        "Using provider: {} ({})",
        provider.name(),
        provider.description()
    );

    let raw_records = provider
        .parse(&cli.source, &loaded.mapping, &loaded.provider, cli.strict)
        .with_context(|| format!("Failed to parse source file: {}", cli.source.display()))?;

    info!("Parsed {} records", raw_records.len());

    let rule_engine = RuleEngine::new(&loaded.provider.rules, &loaded.global);
    let transactions = transform_records(
        provider.as_ref(),
        raw_records,
        &rule_engine,
        &loaded.provider,
        cli.strict,
    )?;

    let writer = BeancountWriter::new(loaded.provider.output.clone());
    let mut output: Box<dyn Write> = match cli.output {
        Some(path) => {
            info!("Writing output to file: {}", path.display());
            Box::new(
                fs::File::create(&path)
                    .with_context(|| format!("Failed to create output file: {}", path.display()))?,
            )
        }
        None => {
            debug!("Writing output to stdout");
            Box::new(io::stdout())
        }
    };

    writer.write(&transactions, &mut output)?;
    info!("Successfully generated {} transactions", transactions.len());

    Ok(())
}

/// Transforms provider records into Beancount transactions and applies
/// deterministic ordering plus lot-cost resolution for sell postings.
fn transform_records(
    provider: &dyn crate::interface::provider::Provider,
    raw_records: Vec<crate::model::data::raw_record::RawRecord>,
    rule_engine: &RuleEngine,
    provider_config: &crate::model::config::provider::ProviderConfig,
    strict_mode: bool,
) -> Result<Vec<Transaction>> {
    let mut success_count = 0usize;
    let mut ignored_count = 0usize;
    let mut error_count = 0usize;
    let mut transactions = Vec::new();

    for (index, raw_record) in raw_records.into_iter().enumerate() {
        match provider.transform(raw_record, rule_engine, provider_config) {
            Ok(Some(transaction)) => {
                success_count += 1;
                debug!(
                    "Record {} transformed: {} {}",
                    index + 1,
                    transaction.date,
                    transaction.narration
                );
                transactions.push(transaction);
            }
            Ok(None) => {
                ignored_count += 1;
                debug!("Record {} ignored by rule", index + 1);
            }
            Err(error) => {
                error_count += 1;

                if strict_mode {
                    return Err(anyhow!(
                        "Record {} transformation failed in strict mode: {}",
                        index + 1,
                        error
                    ));
                }

                warn!("Record {} skipped with error: {}", index + 1, error);
            }
        }
    }

    // Sort output deterministically to keep reconciliation stable across runs.
    sort_transactions_for_output(&mut transactions);
    // Resolve inferred lot costs (`{}`) before writing, so Beancount can
    // unambiguously match sales against known inventory lots.
    let mut seed_inventory = load_seed_inventory_from_files(&provider_config.inventory_seed_files);
    resolve_inferred_cost_postings_with_inventory(&mut transactions, &mut seed_inventory);

    info!(
        "Transformation complete: {} success, {} ignored, {} failed",
        success_count, ignored_count, error_count
    );

    Ok(transactions)
}

#[derive(Debug, Clone)]
struct InventoryLot {
    /// Remaining quantity available in this lot.
    remaining: Decimal,
    /// Cost basis attached to this lot.
    cost: Cost,
}

/// Inventory buckets keyed by `(account, commodity)`.
type InventoryMap = HashMap<(String, String), Vec<InventoryLot>>;

/// Sorts transactions by trade date, then commission/entrust date, then order id.
fn sort_transactions_for_output(transactions: &mut [Transaction]) {
    transactions.sort_by_cached_key(|tx| {
        let commission_date = transaction_commission_date(tx);
        let order_id = transaction_order_id(tx);
        (
            tx.date,
            commission_date.is_none(),
            commission_date,
            order_id.is_none(),
            order_id,
        )
    });
}

/// Returns commission-like date from metadata for secondary sort ordering.
fn transaction_commission_date(tx: &Transaction) -> Option<chrono::NaiveDate> {
    const COMMISSION_DATE_KEYS: [&str; 4] = [
        "commissionDate",
        "commission_date",
        "entrustDate",
        "payTime",
    ];

    for key in COMMISSION_DATE_KEYS {
        let Some(value) = tx.metadata.get(key) else {
            continue;
        };
        if let Some(date) = meta_value_to_date(value) {
            return Some(date);
        }
    }

    None
}

/// Returns order/reference id from metadata for deterministic tie-breaking.
fn transaction_order_id(tx: &Transaction) -> Option<String> {
    const ORDER_ID_KEYS: [&str; 4] = ["orderId", "order_id", "orderid", "reference"];

    for key in ORDER_ID_KEYS {
        let Some(value) = tx.metadata.get(key) else {
            continue;
        };
        let Some(raw) = meta_value_to_string(value) else {
            continue;
        };
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

/// Converts metadata value to string for generic key extraction.
fn meta_value_to_string(value: &MetaValue) -> Option<String> {
    match value {
        MetaValue::String(raw) => Some(raw.clone()),
        MetaValue::Number(raw) => Some(raw.to_string()),
        MetaValue::Date(raw) => Some(raw.format("%Y-%m-%d").to_string()),
        _ => None,
    }
}

/// Converts metadata value to date if it contains date-compatible data.
fn meta_value_to_date(value: &MetaValue) -> Option<chrono::NaiveDate> {
    match value {
        MetaValue::Date(value) => Some(*value),
        MetaValue::String(value) => parse_flexible_date(value),
        MetaValue::Number(value) => parse_flexible_date(&value.to_string()),
        _ => None,
    }
}

/// Parses common date and datetime formats from imported metadata.
fn parse_flexible_date(raw: &str) -> Option<chrono::NaiveDate> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let date_formats = ["%Y%m%d", "%Y-%m-%d", "%Y/%m/%d"];
    for format in date_formats {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(trimmed, format) {
            return Some(date);
        }
    }

    let datetime_formats = ["%Y%m%d%H%M%S", "%Y-%m-%d %H:%M:%S", "%Y/%m/%d %H:%M:%S"];
    for format in datetime_formats {
        if let Ok(datetime) = chrono::NaiveDateTime::parse_from_str(trimmed, format) {
            return Some(datetime.date());
        }
    }

    let digits = trimmed
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.len() >= 8 {
        return chrono::NaiveDate::parse_from_str(&digits[0..8], "%Y%m%d").ok();
    }

    None
}

/// Returns whether a commodity should be treated as fiat cash.
fn is_fiat_currency(currency: &str) -> bool {
    matches!(
        currency,
        "CNY" | "USD" | "HKD" | "EUR" | "JPY" | "GBP" | "SGD" | "CHF" | "AUD" | "CAD"
    )
}

/// Builds one split sell posting with concrete lot cost.
fn build_sell_split_posting(template: &Posting, quantity: Decimal, cost: Cost) -> Option<Posting> {
    let mut posting = template.clone();
    let amount = posting.amount.as_mut()?;

    amount.number = -quantity;
    posting.cost = Some(cost);
    posting.inferred_cost = false;

    Some(posting)
}

/// Builds residual sell posting for unmatched quantity.
fn build_sell_residual_posting(template: &Posting, remaining: Decimal) -> Option<Posting> {
    let mut posting = template.clone();
    let amount = posting.amount.as_mut()?;
    amount.number = -remaining;
    Some(posting)
}

#[cfg(test)]
/// Test-only helper that resolves inferred lots without seed inventory.
fn resolve_inferred_cost_postings(transactions: &mut [Transaction]) {
    let mut inventory: InventoryMap = HashMap::new();
    resolve_inferred_cost_postings_with_inventory(transactions, &mut inventory);
}

/// Rewrites sell postings with inferred or incomplete lot cost into explicit lot splits.
fn resolve_inferred_cost_postings_with_inventory(
    transactions: &mut [Transaction],
    inventory: &mut InventoryMap,
) {
    for tx in transactions {
        let mut rewritten = Vec::with_capacity(tx.postings.len());

        for posting in tx.postings.drain(..) {
            let Some(amount) = &posting.amount else {
                rewritten.push(posting);
                continue;
            };

            if is_fiat_currency(&amount.currency) {
                rewritten.push(posting);
                continue;
            }

            let key = (posting.account.clone(), amount.currency.clone());

            // Buy-side inventory: register lots for later sell matching.
            if amount.number.is_sign_positive() {
                if let Some(cost) = &posting.cost {
                    let mut lot_cost = cost.clone();
                    if lot_cost.date.is_none() {
                        lot_cost.date = Some(tx.date);
                    }
                    inventory.entry(key).or_default().push(InventoryLot {
                        remaining: amount.number,
                        cost: lot_cost,
                    });
                }

                rewritten.push(posting);
                continue;
            }

            // Sell-side splitting targets:
            // 1) inferred cost sell `{}` + optional price
            // 2) explicit cost without lot date, e.g. `{100 CNY}`
            let split_inferred = posting.inferred_cost;
            let split_explicit_without_date = posting
                .cost
                .as_ref()
                .map(|cost| cost.date.is_none())
                .unwrap_or(false);
            if !amount.number.is_sign_negative()
                || (!split_inferred && !split_explicit_without_date)
            {
                rewritten.push(posting);
                continue;
            }

            let lots = inventory.entry(key).or_default();
            let mut remaining = amount.number.abs();
            let mut split_postings = Vec::new();

            for lot in lots.iter_mut() {
                if remaining.is_zero() {
                    break;
                }
                if lot.remaining.is_zero() {
                    continue;
                }

                if !split_inferred {
                    let Some(target_cost) = posting.cost.as_ref() else {
                        continue;
                    };
                    if !cost_matches(&lot.cost, target_cost) {
                        continue;
                    }
                }

                let matched = if lot.remaining <= remaining {
                    lot.remaining
                } else {
                    remaining
                };

                if matched.is_zero() {
                    continue;
                }

                lot.remaining -= matched;
                remaining -= matched;

                if let Some(split) = build_sell_split_posting(&posting, matched, lot.cost.clone()) {
                    split_postings.push(split);
                }
            }

            lots.retain(|lot| !lot.remaining.is_zero());

            if split_postings.is_empty() {
                // No lot in current+seed inventory; keep original posting for downstream booking.
                rewritten.push(posting);
                continue;
            }

            rewritten.extend(split_postings);

            // Keep unresolved residual, if any.
            if !remaining.is_zero()
                && let Some(residual) = build_sell_residual_posting(&posting, remaining)
            {
                rewritten.push(residual);
            }
        }

        tx.postings = rewritten;
    }
}

/// Loads inventory seed files and merges them into an in-memory lot map.
fn load_seed_inventory_from_files(paths: &[String]) -> InventoryMap {
    if paths.is_empty() {
        return HashMap::new();
    }

    let mut inventory = HashMap::new();
    for path in paths {
        let seed_path = Path::new(path);
        match ingest_seed_inventory_file(seed_path, &mut inventory) {
            Ok(()) => debug!("Loaded inventory seed file: {}", seed_path.display()),
            Err(error) => warn!(
                "Failed to load inventory seed file '{}': {}",
                seed_path.display(),
                error
            ),
        }
    }
    inventory
}

/// Parses one Beancount seed file and applies its postings to the inventory map.
fn ingest_seed_inventory_file(path: &Path, inventory: &mut InventoryMap) -> Result<()> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read inventory seed file: {}", path.display()))?;

    let mut current_date: Option<chrono::NaiveDate> = None;

    for line in content.lines() {
        if let Some(tx_date) = parse_seed_transaction_date(line) {
            current_date = Some(tx_date);
            continue;
        }

        let Some(parsed) = parse_seed_posting_line(line, current_date) else {
            continue;
        };

        if is_fiat_currency(&parsed.commodity) {
            continue;
        }

        let key = (parsed.account, parsed.commodity);
        let lots = inventory.entry(key).or_default();

        if parsed.quantity.is_sign_positive() {
            let Some(mut cost) = parsed.cost else {
                continue;
            };
            if cost.date.is_none() {
                cost.date = current_date;
            }
            lots.push(InventoryLot {
                remaining: parsed.quantity,
                cost,
            });
            continue;
        }

        if !parsed.quantity.is_sign_negative() {
            continue;
        }

        let Some(target_cost) = parsed.cost else {
            continue;
        };

        let mut remaining = parsed.quantity.abs();
        for lot in lots.iter_mut() {
            if remaining.is_zero() {
                break;
            }
            if lot.remaining.is_zero() {
                continue;
            }
            if !cost_matches(&lot.cost, &target_cost) {
                continue;
            }

            let matched = if lot.remaining <= remaining {
                lot.remaining
            } else {
                remaining
            };
            lot.remaining -= matched;
            remaining -= matched;
        }
        lots.retain(|lot| !lot.remaining.is_zero());
    }

    Ok(())
}

#[derive(Debug)]
struct ParsedSeedPosting {
    account: String,
    quantity: Decimal,
    commodity: String,
    cost: Option<Cost>,
}

/// Parses `YYYY-MM-DD *` / `YYYY-MM-DD !` transaction headers from seed files.
fn parse_seed_transaction_date(line: &str) -> Option<chrono::NaiveDate> {
    static TX_DATE_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^(?P<date>\d{4}-\d{2}-\d{2})\s+[*!]").expect("valid tx date regex")
    });

    let caps = TX_DATE_RE.captures(line.trim())?;
    chrono::NaiveDate::parse_from_str(caps.name("date")?.as_str(), "%Y-%m-%d").ok()
}

/// Parses a posting line with optional cost expression from seed files.
fn parse_seed_posting_line(
    line: &str,
    fallback_date: Option<chrono::NaiveDate>,
) -> Option<ParsedSeedPosting> {
    static POSTING_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r#"^\s{2}(?:[*!]\s+)?(?P<account>\S+)\s+(?P<number>[+-]?\d+(?:\.\d+)?)\s+(?P<commodity>[A-Za-z0-9_.-]+)(?:\s+\{(?P<cost>[^}]*)\})?"#,
        )
        .expect("valid posting regex")
    });

    let caps = POSTING_RE.captures(line)?;
    let account = caps.name("account")?.as_str().to_string();
    let quantity = Decimal::from_str(caps.name("number")?.as_str()).ok()?;
    let commodity = caps.name("commodity")?.as_str().to_string();
    let cost = caps
        .name("cost")
        .and_then(|raw| parse_seed_cost(raw.as_str(), fallback_date));

    Some(ParsedSeedPosting {
        account,
        quantity,
        commodity,
        cost,
    })
}

/// Parses cost payload from `{number currency[, date][, "label"]}`.
fn parse_seed_cost(raw: &str, fallback_date: Option<chrono::NaiveDate>) -> Option<Cost> {
    static COST_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r#"^\s*(?P<number>[+-]?\d+(?:\.\d+)?)\s+(?P<currency>[A-Za-z0-9_.-]+)(?:,\s*(?P<date>\d{4}-\d{2}-\d{2}))?(?:,\s*\"(?P<label>[^\"]*)\")?\s*$"#,
        )
        .expect("valid cost regex")
    });

    let caps = COST_RE.captures(raw.trim())?;
    let number = Decimal::from_str(caps.name("number")?.as_str()).ok()?;
    let currency = caps.name("currency")?.as_str().to_string();
    let mut cost = Cost::new(number, currency);

    if let Some(date_match) = caps.name("date") {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_match.as_str(), "%Y-%m-%d") {
            cost.date = Some(date);
        }
    } else {
        cost.date = fallback_date;
    }

    if let Some(label_match) = caps.name("label") {
        cost.label = Some(label_match.as_str().to_string());
    }

    Some(cost)
}

/// Returns whether an existing lot cost satisfies the target cost constraint.
fn cost_matches(lot_cost: &Cost, target_cost: &Cost) -> bool {
    let same_number = lot_cost.number == target_cost.number;
    let same_currency = lot_cost.currency == target_cost.currency;
    let same_label = lot_cost.label == target_cost.label;
    let same_date = target_cost.date.is_none() || lot_cost.date == target_cost.date;
    same_number && same_currency && same_label && same_date
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    use crate::{
        error::{ImporterError, ImporterResult},
        interface::provider::Provider,
        model::{
            account::{amount::Amount, cost::Cost, posting::Posting, price::Price},
            config::{global::GlobalConfig, meta_value::MetaValue, provider::ProviderConfig},
            data::raw_record::RawRecord,
            mapping::field_mapping::FieldMapping,
            rule::{Rule, rule_engine::RuleEngine},
            transaction::Transaction,
        },
    };

    use super::{
        load_seed_inventory_from_files, resolve_inferred_cost_postings,
        resolve_inferred_cost_postings_with_inventory, sort_transactions_for_output,
        transform_records,
    };

    struct AlwaysFailProvider;

    impl Provider for AlwaysFailProvider {
        fn name(&self) -> &'static str {
            "always-fail"
        }

        fn parse(
            &self,
            _path: &std::path::Path,
            _mapping: &FieldMapping,
            _config: &ProviderConfig,
            _strict_mode: bool,
        ) -> ImporterResult<Vec<RawRecord>> {
            Ok(vec![])
        }

        fn transform(
            &self,
            _record: RawRecord,
            _rule_engine: &RuleEngine,
            _config: &ProviderConfig,
        ) -> ImporterResult<Option<Transaction>> {
            Err(ImporterError::Conversion("mock failure".to_string()))
        }
    }

    fn build_rule_engine() -> RuleEngine<'static> {
        let provider_rules: &'static [Rule] = Box::leak(Vec::<Rule>::new().into_boxed_slice());
        let global: &'static GlobalConfig = Box::leak(Box::new(GlobalConfig::default()));
        RuleEngine::new(provider_rules, global)
    }

    #[test]
    fn strict_mode_returns_error_on_transform_failure() {
        let provider = AlwaysFailProvider;
        let records = vec![RawRecord::new()];
        let rule_engine = build_rule_engine();
        let provider_config = ProviderConfig::default();

        let result = transform_records(&provider, records, &rule_engine, &provider_config, true);
        assert!(result.is_err());
    }

    #[test]
    fn non_strict_mode_skips_transform_failure() {
        let provider = AlwaysFailProvider;
        let records = vec![RawRecord::new()];
        let rule_engine = build_rule_engine();
        let provider_config = ProviderConfig::default();

        let result = transform_records(&provider, records, &rule_engine, &provider_config, false)
            .expect("non-strict mode should not fail");

        assert!(result.is_empty());
    }

    #[test]
    fn sorts_by_trade_date_then_commission_date_ascending() {
        let tx_older_date = Transaction::new(
            NaiveDate::from_ymd_opt(2026, 1, 3).expect("valid date"),
            "older-date",
        )
        .with_meta("commissionDate", MetaValue::String("20260109".to_string()));

        let tx_same_date_commission_1 = Transaction::new(
            NaiveDate::from_ymd_opt(2026, 1, 4).expect("valid date"),
            "same-date-commission-1",
        )
        .with_meta("commissionDate", MetaValue::String("20260108".to_string()))
        .with_meta("orderId", MetaValue::String("002".to_string()));

        let tx_same_date_commission_2 = Transaction::new(
            NaiveDate::from_ymd_opt(2026, 1, 4).expect("valid date"),
            "same-date-commission-2",
        )
        .with_meta("commissionDate", MetaValue::String("20260107".to_string()))
        .with_meta("orderId", MetaValue::String("001".to_string()));

        let tx_same_date_without_commission = Transaction::new(
            NaiveDate::from_ymd_opt(2026, 1, 4).expect("valid date"),
            "same-date-no-commission",
        )
        .with_meta("orderId", MetaValue::String("003".to_string()));

        let mut transactions = vec![
            tx_same_date_without_commission,
            tx_same_date_commission_1,
            tx_same_date_commission_2,
            tx_older_date,
        ];

        sort_transactions_for_output(&mut transactions);

        let ordered_narrations = transactions
            .iter()
            .map(|tx| tx.narration.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ordered_narrations,
            vec![
                "older-date",
                "same-date-commission-2",
                "same-date-commission-1",
                "same-date-no-commission",
            ]
        );
    }

    #[test]
    fn resolves_inferred_sell_into_explicit_fifo_lots() {
        let buy_1 = Transaction::new(
            NaiveDate::from_ymd_opt(2025, 12, 23).expect("valid date"),
            "buy lot 1",
        )
        .with_posting(
            Posting::new("Assets:Invest:Broker:Securities")
                .with_amount(Amount::new(dec!(275), "FUND_161226"))
                .with_cost(Cost::new(dec!(1.7987), "CNY")),
        );

        let buy_2 = Transaction::new(
            NaiveDate::from_ymd_opt(2025, 12, 24).expect("valid date"),
            "buy lot 2",
        )
        .with_posting(
            Posting::new("Assets:Invest:Broker:Securities")
                .with_amount(Amount::new(dec!(267), "FUND_161226"))
                .with_cost(Cost::new(dec!(1.8527), "CNY")),
        );

        let sell = Transaction::new(
            NaiveDate::from_ymd_opt(2025, 12, 26).expect("valid date"),
            "sell mixed lots",
        )
        .with_posting(
            Posting::new("Assets:Invest:Broker:Securities")
                .with_amount(Amount::new(dec!(-523), "FUND_161226"))
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
                .with_amount(Amount::new(dec!(100), "FUND_161226"))
                .with_cost(Cost::new(dec!(1.7987), "CNY")),
        );

        let sell = Transaction::new(
            NaiveDate::from_ymd_opt(2025, 12, 24).expect("valid date"),
            "sell more than current file lots",
        )
        .with_posting(
            Posting::new("Assets:Invest:Broker:Securities")
                .with_amount(Amount::new(dec!(-150), "FUND_161226"))
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
  Assets:Invest:Broker:Securities  154 FUND_161226 {1.9469 CNY}
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
                    .with_amount(Amount::new(dec!(-100), "FUND_161226"))
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

        let _ = fs::remove_file(PathBuf::from(seed_path));
    }
}
