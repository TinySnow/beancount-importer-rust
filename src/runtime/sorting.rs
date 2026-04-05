//! 交易输出排序模块。
//!
//! 该模块负责在写出前为交易建立确定性排序规则，保证重复导入时输出顺序稳定，
//! 从而降低版本差异噪音并提升对账可读性。
//!
//! 当前排序优先级如下：
//! 1. 交易日期 `tx.date`；
//! 2. 委托/成交日期（metadata）；
//! 3. 订单号或引用号（metadata）。
//!
//! 其中 metadata 的字段名支持多来源别名，以兼容不同券商/银行导出格式。

use crate::model::{config::meta_value::MetaValue, transaction::Transaction};

/// 按交易日期、委托/成交日期、订单号对交易进行稳定排序。
///
/// # 参数
/// - `transactions`：待排序的交易切片，会被原地重排。
///
/// # 排序规则
/// - 主键：`tx.date`；
/// - 次键：`commissionDate` 等委托/成交日期；
/// - 末键：`orderId` 等订单号字段。
///
/// 对于缺失次键/末键的交易，排序时会自动排在已提供键值的交易之后。
pub(crate) fn sort_transactions_for_output(transactions: &mut [Transaction]) {
    transactions.sort_by_cached_key(|tx| {
        let commission_date = transaction_commission_date(tx);
        let order_id = transaction_order_id(tx);
        // `is_none()` 维度用于将 `Some(..)` 排在 `None` 前，避免缺失字段的记录
        // 抢占更完整记录的顺序位置，从而提升同日交易排序稳定性。
        (
            tx.date,
            commission_date.is_none(),
            commission_date,
            order_id.is_none(),
            order_id,
        )
    });
}

/// 从交易 metadata 提取“委托/成交类日期”，作为二级排序键。
///
/// 按内置字段优先顺序查找，读取到首个可解析日期即返回。
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

/// 从交易 metadata 提取订单号/引用号，作为同日同委托日期下的稳定打散键。
///
/// 返回值会自动去除首尾空白；空字符串视为无效值。
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

/// 将元数据值转为字符串表示，供排序键提取逻辑复用。
///
/// 仅对字符串、数字、日期类型返回结果，其余类型返回 `None`。
fn meta_value_to_string(value: &MetaValue) -> Option<String> {
    match value {
        MetaValue::String(raw) => Some(raw.clone()),
        MetaValue::Number(raw) => Some(raw.to_string()),
        MetaValue::Date(raw) => Some(raw.format("%Y-%m-%d").to_string()),
        _ => None,
    }
}

/// 将元数据值解析为日期。
///
/// 支持元数据原生日期类型，以及可转换为日期的字符串/数字内容。
fn meta_value_to_date(value: &MetaValue) -> Option<chrono::NaiveDate> {
    match value {
        MetaValue::Date(value) => Some(*value),
        MetaValue::String(value) => parse_flexible_date(value),
        MetaValue::Number(value) => parse_flexible_date(&value.to_string()),
        _ => None,
    }
}

/// 解析导入元数据中常见的日期与日期时间格式。
///
/// 解析顺序：
/// 1. 常见纯日期格式；
/// 2. 常见日期时间格式（取日期部分）；
/// 3. 从混合字符串中提取前 8 位数字按 `YYYYMMDD` 兜底解析。
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

    // 兜底逻辑：部分券商字段可能混入分隔符或附加文本，提取纯数字后再尝试解析。
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
