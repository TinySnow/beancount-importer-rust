use std::{collections::HashMap, str::FromStr};

use rust_decimal::Decimal;

use crate::model::{config::meta_value::MetaValue, transaction::Transaction};

use super::currency::is_fiat_currency;

#[derive(Debug, Clone, Copy)]
struct TradeProfitMetadata {
    gross_pnl: Decimal,
    fee_total: Decimal,
    net_pnl: Decimal,
}

/// 计算并写入交易级收益元数据：
/// - `grossPnl`：税费前已实现收益（基于卖出 lot 成本）；
/// - `feeTotal`：本笔交易税费合计；
/// - `netPnl`：`grossPnl - feeTotal`。
///
/// 说明：
/// - 这里是“逐笔值”，不是历史累计值；
/// - 卖出 lot 成本无法完整确定时，不写入收益元数据，避免误导。
pub(crate) fn annotate_trade_profit_metadata(transactions: &mut [Transaction]) {
    for tx in transactions {
        let Some(pnl) = calculate_trade_profit_metadata(tx) else {
            continue;
        };
        tx.metadata
            .insert("grossPnl".to_string(), MetaValue::Number(pnl.gross_pnl));
        tx.metadata
            .insert("feeTotal".to_string(), MetaValue::Number(pnl.fee_total));
        tx.metadata
            .insert("netPnl".to_string(), MetaValue::Number(pnl.net_pnl));
    }
}

/// 基于标准化分录计算单笔交易收益元数据。
fn calculate_trade_profit_metadata(tx: &Transaction) -> Option<TradeProfitMetadata> {
    let mut has_non_fiat_posting = false;
    let mut has_sell_posting = false;
    let mut unresolved_sell = false;
    let mut quote_currency: Option<&str> = None;
    let mut gross_pnl = Decimal::ZERO;

    for posting in &tx.postings {
        let Some(amount) = &posting.amount else {
            continue;
        };
        if is_fiat_currency(&amount.currency) {
            continue;
        }

        has_non_fiat_posting = true;

        if !amount.number.is_sign_negative() {
            continue;
        }

        has_sell_posting = true;
        let (Some(cost), Some(price)) = (&posting.cost, &posting.price) else {
            // 卖出 lot 信息不完整时，grossPnl 会失真，因此直接标记为未解析。
            unresolved_sell = true;
            continue;
        };

        let quantity = amount.number.abs();
        gross_pnl += quantity * (price.number - cost.number);
        if quote_currency.is_none() {
            quote_currency = Some(price.currency.as_str());
        }
    }

    if !has_non_fiat_posting {
        return None;
    }

    if has_sell_posting && unresolved_sell {
        return None;
    }

    let explicit_fee_total = read_numeric_metadata(&tx.metadata, "fee").unwrap_or(Decimal::ZERO)
        + read_numeric_metadata(&tx.metadata, "tax").unwrap_or(Decimal::ZERO);
    let inferred_fee_total = infer_fee_total_from_postings(tx, quote_currency);
    if !has_sell_posting
        && gross_pnl.is_zero()
        && explicit_fee_total.is_zero()
        && inferred_fee_total.is_zero()
    {
        return None;
    }
    let fee_total = if explicit_fee_total.is_zero() {
        inferred_fee_total
    } else {
        explicit_fee_total
    };
    let net_pnl = gross_pnl - fee_total;

    Some(TradeProfitMetadata {
        gross_pnl,
        fee_total,
        net_pnl,
    })
}

fn infer_fee_total_from_postings(tx: &Transaction, quote_currency: Option<&str>) -> Decimal {
    tx.postings
        .iter()
        .filter(|posting| posting.account.starts_with("Expenses:"))
        .filter_map(|posting| posting.amount.as_ref())
        .filter(|amount| amount.number.is_sign_positive())
        .filter(|amount| match quote_currency {
            Some(currency) => amount.currency == currency,
            None => is_fiat_currency(&amount.currency),
        })
        .map(|amount| amount.number)
        .fold(Decimal::ZERO, |acc, number| acc + number)
}

fn read_numeric_metadata(metadata: &HashMap<String, MetaValue>, key: &str) -> Option<Decimal> {
    let value = metadata.get(key)?;
    match value {
        MetaValue::Number(number) => Some(*number),
        MetaValue::String(raw) => Decimal::from_str(raw.trim()).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    use crate::model::{
        account::{amount::Amount, cost::Cost, posting::Posting, price::Price},
        config::meta_value::MetaValue,
        transaction::Transaction,
    };

    use super::annotate_trade_profit_metadata;

    fn metadata_number(tx: &Transaction, key: &str) -> Option<Decimal> {
        match tx.metadata.get(key) {
            Some(MetaValue::Number(value)) => Some(*value),
            _ => None,
        }
    }

    #[test]
    fn annotates_trade_profit_metadata_for_buy_and_sell() {
        let buy = Transaction::new(
            NaiveDate::from_ymd_opt(2025, 12, 2).expect("valid date"),
            "security buy",
        )
        .with_posting(
            Posting::new("Assets:Invest:Broker:Securities")
                .with_amount(Amount::new(dec!(100), "SEC_159915"))
                .with_cost(Cost::new(dec!(3.06), "CNY")),
        )
        .with_posting(
            Posting::new("Assets:Invest:Broker:Cash").with_amount(Amount::new(dec!(-306.1), "CNY")),
        )
        .with_posting(
            Posting::new("Expenses:Finance:Trading:Fee").with_amount(Amount::new(dec!(0.1), "CNY")),
        );

        let sell = Transaction::new(
            NaiveDate::from_ymd_opt(2025, 12, 5).expect("valid date"),
            "security sell",
        )
        .with_posting(
            Posting::new("Assets:Invest:Broker:Securities")
                .with_amount(Amount::new(dec!(-100), "SEC_159915"))
                .with_cost(Cost::new(dec!(3.06), "CNY"))
                .with_price(Price::new(dec!(3.07), "CNY")),
        )
        .with_posting(
            Posting::new("Assets:Invest:Broker:Cash").with_amount(Amount::new(dec!(306.9), "CNY")),
        )
        .with_posting(
            Posting::new("Expenses:Finance:Trading:Fee").with_amount(Amount::new(dec!(0.1), "CNY")),
        )
        .with_posting(Posting::new("Income:Finance:Trading:PnL"));

        let mut transactions = vec![buy, sell];
        annotate_trade_profit_metadata(&mut transactions);

        assert_eq!(metadata_number(&transactions[0], "grossPnl"), Some(dec!(0)));
        assert_eq!(
            metadata_number(&transactions[0], "feeTotal"),
            Some(dec!(0.1))
        );
        assert_eq!(
            metadata_number(&transactions[0], "netPnl"),
            Some(dec!(-0.1))
        );

        assert_eq!(
            metadata_number(&transactions[1], "grossPnl"),
            Some(dec!(1.0))
        );
        assert_eq!(
            metadata_number(&transactions[1], "feeTotal"),
            Some(dec!(0.1))
        );
        assert_eq!(metadata_number(&transactions[1], "netPnl"), Some(dec!(0.9)));
    }

    #[test]
    fn prefers_explicit_fee_and_tax_metadata_when_present() {
        let sell = Transaction::new(
            NaiveDate::from_ymd_opt(2025, 12, 5).expect("valid date"),
            "security sell",
        )
        .with_posting(
            Posting::new("Assets:Invest:Broker:Securities")
                .with_amount(Amount::new(dec!(-100), "SEC_159915"))
                .with_cost(Cost::new(dec!(3.06), "CNY"))
                .with_price(Price::new(dec!(3.07), "CNY")),
        )
        .with_posting(
            Posting::new("Assets:Invest:Broker:Cash").with_amount(Amount::new(dec!(306.88), "CNY")),
        )
        .with_posting(
            Posting::new("Expenses:Finance:Trading:Fee")
                .with_amount(Amount::new(dec!(0.12), "CNY")),
        )
        .with_meta("fee", MetaValue::Number(dec!(0.1)))
        .with_meta("tax", MetaValue::Number(dec!(0.02)));

        let mut transactions = vec![sell];
        annotate_trade_profit_metadata(&mut transactions);

        assert_eq!(
            metadata_number(&transactions[0], "grossPnl"),
            Some(dec!(1.0))
        );
        assert_eq!(
            metadata_number(&transactions[0], "feeTotal"),
            Some(dec!(0.12))
        );
        assert_eq!(
            metadata_number(&transactions[0], "netPnl"),
            Some(dec!(0.88))
        );
    }

    #[test]
    fn skips_profit_metadata_when_sell_lot_is_unresolved() {
        let unresolved_sell = Transaction::new(
            NaiveDate::from_ymd_opt(2025, 12, 5).expect("valid date"),
            "unresolved sell",
        )
        .with_posting(
            Posting::new("Assets:Invest:Broker:Securities")
                .with_amount(Amount::new(dec!(-100), "SEC_159915"))
                .with_inferred_cost()
                .with_price(Price::new(dec!(3.07), "CNY")),
        )
        .with_posting(
            Posting::new("Assets:Invest:Broker:Cash").with_amount(Amount::new(dec!(306.9), "CNY")),
        )
        .with_posting(
            Posting::new("Expenses:Finance:Trading:Fee").with_amount(Amount::new(dec!(0.1), "CNY")),
        );

        let mut transactions = vec![unresolved_sell];
        annotate_trade_profit_metadata(&mut transactions);

        assert_eq!(metadata_number(&transactions[0], "grossPnl"), None);
        assert_eq!(metadata_number(&transactions[0], "feeTotal"), None);
        assert_eq!(metadata_number(&transactions[0], "netPnl"), None);
    }
}
