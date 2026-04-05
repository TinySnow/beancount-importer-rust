//! 日期与日期时间解析工具。
//!
//! 支持常见账单导出格式：
//! - `YYYY/MM/DD HH:MM:SS`
//! - `YYYY-MM-DD HH:MM:SS`
//! - `YYYY/MM/DD`
//! - `YYYY-MM-DD`
//!
//! 缺失时间部分时，默认补 `00:00:00`。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::utils::date::{parse_date, parse_datetime};
//!
//! let dt = parse_datetime("2023/12/31 3:44:00").unwrap();
//! assert_eq!(dt.format("%Y-%m-%d %H:%M:%S").to_string(), "2023-12-31 03:44:00");
//!
//! let date = parse_date("2023-12-31").unwrap();
//! assert_eq!(date.to_string(), "2023-12-31");
//! ```

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use log::trace;

/// 解析日期时间字符串。
///
/// 支持示例：
/// - `2023/12/31 3:44:00`
/// - `2023-12-31 13:44:00`
/// - `2023/12/31`
/// - `2023-12-31`
///
/// # 参数
/// - `s`：原始日期时间文本。
///
/// # 返回值
/// - 成功时返回 [`NaiveDateTime`](chrono::NaiveDateTime)；
/// - 失败时返回 `None`。
///
/// # 示例
/// ```rust
/// use beancount_importer_rust::utils::date::parse_datetime;
///
/// let dt = parse_datetime("2023-12-31 13:44:00").unwrap();
/// assert_eq!(dt.to_string(), "2023-12-31 13:44:00");
///
/// let date_only = parse_datetime("2023/12/31").unwrap();
/// assert_eq!(date_only.to_string(), "2023-12-31 00:00:00");
/// ```
pub fn parse_datetime(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // 允许日期和时间之间出现多个空白字符。
    let parts: Vec<&str> = s.split_whitespace().collect();

    let date = parse_date_part(parts.first()?)?;
    // 缺失时间时按 00:00:00 处理，保证仅日期文本可被接受。
    let time = parts
        .get(1)
        .and_then(|s: &&str| parse_time_part(s))
        .unwrap_or(NaiveTime::MIN);

    Some(NaiveDateTime::new(date, time))
}

/// 仅解析日期部分。
///
/// 内部复用 [`parse_datetime`]，因此支持同样的输入格式。
///
/// # 示例
/// ```rust
/// use beancount_importer_rust::utils::date::parse_date;
///
/// assert_eq!(
///     parse_date("2023/12/31 3:44:00").map(|d| d.to_string()),
///     Some("2023-12-31".to_string())
/// );
/// ```
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    parse_datetime(s).map(|dt| dt.date())
}

/// 解析日期文本（`YYYY/MM/DD` 或 `YYYY-MM-DD`）。
fn parse_date_part(s: &str) -> Option<NaiveDate> {
    let parts: Vec<&str> = s.split(['/', '-']).collect();

    if parts.len() != 3 {
        trace!("日期格式错误: {}", s);
        return None;
    }

    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;

    NaiveDate::from_ymd_opt(year, month, day)
}

/// 解析时间文本（`HH:MM` 或 `HH:MM:SS`）。
fn parse_time_part(s: &str) -> Option<NaiveTime> {
    let parts: Vec<&str> = s.split(':').collect();

    if parts.len() < 2 {
        trace!("时间格式错误: {}", s);
        return None;
    }

    let hour: u32 = parts[0].parse().ok()?;
    let min: u32 = parts[1].parse().ok()?;
    let sec: u32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

    NaiveTime::from_hms_opt(hour, min, sec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alipay_format() {
        let dt = parse_datetime("2023/12/31  3:44:00").unwrap();
        assert_eq!(dt.to_string(), "2023-12-31 03:44:00");
    }

    #[test]
    fn test_standard_format() {
        let dt = parse_datetime("2023-12-31 13:44:00").unwrap();
        assert_eq!(dt.to_string(), "2023-12-31 13:44:00");
    }

    #[test]
    fn test_date_only() {
        let dt = parse_datetime("2023/12/31").unwrap();
        assert_eq!(dt.to_string(), "2023-12-31 00:00:00");
    }

    #[test]
    fn test_parse_date() {
        let d = parse_date("2023/12/31 3:44:00").unwrap();
        assert_eq!(d.to_string(), "2023-12-31");
    }
}
