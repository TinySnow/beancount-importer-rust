//! 交易盈亏元数据计算模块。
//!
//! 该模块在交易标准化与 lot 成本补全之后运行，负责按“逐笔交易”写入：
//! - `grossPnl`：税费前已实现收益；
//! - `feeTotal`：本笔交易税费合计；
//! - `netPnl`：`grossPnl - feeTotal`。
//!
//! 设计约束：
//! - 仅计算单笔值，不维护历史累计值；
//! - 卖出 lot 信息不完整时，宁可不写入，也不输出可能误导的收益值；
//! - 显式元数据 `fee`/`tax` 优先于从分录自动推断的费用合计。

use std::{collections::HashMap, str::FromStr};

use rust_decimal::Decimal;

use crate::model::{config::meta_value::MetaValue, transaction::Transaction};

use super::currency::is_fiat_currency;

/// 单笔交易收益字段的内部聚合结果。
#[derive(Debug, Clone, Copy)]
struct TradeProfitMetadata {
    /// 税费前已实现收益。
    gross_pnl: Decimal,
    /// 税费总额。
    fee_total: Decimal,
    /// 税后净收益。
    net_pnl: Decimal,
}

/// 计算并写入交易级收益元数据：
/// - `grossPnl`：税费前已实现收益（基于卖出 lot 成本）；
/// - `feeTotal`：本笔交易税费合计；
/// - `netPnl`：`grossPnl - feeTotal`。
///
/// # 参数
/// - `transactions`：待处理交易列表，会被原地补充 metadata。
///
/// # 说明
/// - 这里写入的是“逐笔值”，不是历史累计值；
/// - 若当前交易无法可靠计算收益，则保持原 metadata，不写入上述键。
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
///
/// # 返回值
/// - `Some(TradeProfitMetadata)`：可以可靠计算收益；
/// - `None`：不应写入收益元数据（例如非证券交易或卖出 lot 信息缺失）。
fn calculate_trade_profit_metadata(tx: &Transaction) -> Option<TradeProfitMetadata> {
    // 状态位用于在单次遍历中记录“是否为证券交易”“是否涉及卖出”“卖出信息是否完整”。
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
    // 约定：显式 fee/tax 元数据优先。仅在二者都未给出时才使用推断值。
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

/// 从费用类分录中推断税费总额。
///
/// 推断范围：
/// - 账户前缀为 `Expenses:`；
/// - 金额为正数（支出方向）；
/// - 币种与报价币一致；若未知报价币，则接受法币现金。
///
/// # 参数
/// - `tx`：单笔交易；
/// - `quote_currency`：卖出分录价格币种（可选）。
///
/// # 返回值
/// 推断得到的费用合计，默认 `0`。
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

/// 从 metadata 中读取数值类型字段。
///
/// 支持两种输入：
/// - `MetaValue::Number`；
/// - 可解析为十进制数值的 `MetaValue::String`。
///
/// 其余类型或解析失败时返回 `None`。
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
