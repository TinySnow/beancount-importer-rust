use crate::model::{config::meta_value::MetaValue, transaction::Transaction};

/// 按交易日期、委托/成交日期、订单号排序交易。
///
/// 排序键保持确定性，确保重复导入时输出稳定，便于差异比对与对账。
pub(crate) fn sort_transactions_for_output(transactions: &mut [Transaction]) {
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

/// 从 metadata 提取“委托/成交类日期”，用于二级排序。
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

/// 从 metadata 提取订单号/引用号，用于同日同委托日期下的稳定打散。
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

/// 将元数据值统一转为字符串，供通用键提取使用。
fn meta_value_to_string(value: &MetaValue) -> Option<String> {
    match value {
        MetaValue::String(raw) => Some(raw.clone()),
        MetaValue::Number(raw) => Some(raw.to_string()),
        MetaValue::Date(raw) => Some(raw.format("%Y-%m-%d").to_string()),
        _ => None,
    }
}

/// 将元数据值解析为日期（若内容可解析）。
fn meta_value_to_date(value: &MetaValue) -> Option<chrono::NaiveDate> {
    match value {
        MetaValue::Date(value) => Some(*value),
        MetaValue::String(value) => parse_flexible_date(value),
        MetaValue::Number(value) => parse_flexible_date(&value.to_string()),
        _ => None,
    }
}

/// 解析导入元数据中常见日期/日期时间格式。
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

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use crate::model::{config::meta_value::MetaValue, transaction::Transaction};

    use super::sort_transactions_for_output;

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
}
