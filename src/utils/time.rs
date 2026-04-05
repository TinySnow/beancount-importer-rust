//! 时间文本规范化与 Excel 序列时间工具。
//!
//! 该模块用于处理来源不一致的时间文本：
//! - 纯时间（`14:37` / `14:37:15`）
//! - 日期时间文本（`2026-03-06 14:37:15`）
//! - Excel 序列值（如 `46110.5`）
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::utils::time::{format_excel_datetime_serial, normalize_time_text};
//!
//! assert_eq!(normalize_time_text("14:37"), Some("14:37:00".to_string()));
//! assert_eq!(normalize_time_text("46110.5"), Some("12:00:00".to_string()));
//! assert_eq!(format_excel_datetime_serial(46087.0), "2026-03-06");
//! ```

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime};

/// 规范化时间文本为 `HH:MM:SS`。
///
/// 支持：
/// - Excel 序列时间（如 `46110.5`）；
/// - 日期时间字符串（如 `2026-03-06 14:37:15`）；
/// - 时间字符串（如 `14:37` / `14:37:15`）。
///
/// # 参数
/// - `raw`：原始时间文本。
///
/// # 返回值
/// 成功返回统一后的 `HH:MM:SS`，否则返回 `None`。
///
/// # 示例
/// ```rust
/// use beancount_importer_rust::utils::time::normalize_time_text;
///
/// assert_eq!(normalize_time_text("2026-03-06 14:37"), Some("14:37:00".to_string()));
/// assert_eq!(normalize_time_text("14:37:15"), Some("14:37:15".to_string()));
/// assert_eq!(normalize_time_text(""), None);
/// ```
pub fn normalize_time_text(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(time) = parse_excel_serial_time(trimmed) {
        return Some(time.format("%H:%M:%S").to_string());
    }

    const DATETIME_FORMATS: [&str; 7] = [
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y%m%d%H%M%S",
    ];
    for format in DATETIME_FORMATS {
        if let Ok(value) = NaiveDateTime::parse_from_str(trimmed, format) {
            return Some(value.time().format("%H:%M:%S").to_string());
        }
    }

    const TIME_FORMATS: [&str; 3] = ["%H:%M:%S", "%H:%M", "%H:%M:%S%.f"];
    for format in TIME_FORMATS {
        if let Ok(value) = NaiveTime::parse_from_str(trimmed, format) {
            return Some(value.format("%H:%M:%S").to_string());
        }
    }

    None
}

/// 解析 Excel 序列值中的时间分量。
///
/// Excel 把一天表示为 `1.0`，小数部分对应一天中的时间比例。
///
/// # 参数
/// - `value`：Excel 序列文本。
///
/// # 返回值
/// 若存在有效小数时间分量，返回对应 [`NaiveTime`](chrono::NaiveTime)。
///
/// # 示例
/// ```rust
/// use beancount_importer_rust::utils::time::parse_excel_serial_time;
///
/// assert_eq!(
///     parse_excel_serial_time("46110.5").map(|t| t.format("%H:%M:%S").to_string()),
///     Some("12:00:00".to_string())
/// );
/// assert_eq!(parse_excel_serial_time("46110"), None);
/// ```
pub fn parse_excel_serial_time(value: &str) -> Option<NaiveTime> {
    let serial: f64 = value.parse().ok()?;
    if !serial.is_finite() || serial <= 0.0 {
        return None;
    }

    let fraction = serial.fract().abs();
    if fraction == 0.0 {
        return None;
    }

    // 四舍五入到秒，避免浮点表示误差造成 `xx:xx:59.999...` 偏移。
    let mut seconds = (fraction * 86_400.0).round() as u32;
    // 极端情况下会被舍入到 86400 秒（次日 00:00:00），此处回绕到当天起点。
    if seconds >= 86_400 {
        seconds = 0;
    }

    NaiveTime::from_num_seconds_from_midnight_opt(seconds, 0)
}

/// 解析 Excel 数值序列日期（1900 日期系统）。
///
/// 仅接受 1970~2200 年之间的日期，避免把普通数字误判为日期。
///
/// # 示例
/// ```rust
/// use beancount_importer_rust::utils::time::parse_excel_serial_date;
///
/// assert_eq!(
///     parse_excel_serial_date("46000").map(|d| d.to_string()),
///     Some("2025-12-09".to_string())
/// );
/// assert_eq!(parse_excel_serial_date("100"), None);
/// ```
pub fn parse_excel_serial_date(value: &str) -> Option<NaiveDate> {
    if value.trim().is_empty() {
        return None;
    }

    let serial: f64 = value.parse().ok()?;
    if !serial.is_finite() || serial < 1.0 {
        return None;
    }

    let excel_epoch = NaiveDate::from_ymd_opt(1899, 12, 30)?;
    let day_count = serial.trunc() as i64;
    let date = excel_epoch.checked_add_signed(Duration::days(day_count))?;

    if !(1970..=2200).contains(&date.year()) {
        return None;
    }

    Some(date)
}

/// 将 Excel 日期时间序列值格式化为稳定文本。
///
/// 输出规则：
/// - 仅有时间分量：`HH:MM:SS`
/// - 日期+时间：`YYYY-MM-DD HH:MM:SS`
/// - 仅日期：`YYYY-MM-DD`
///
/// # 示例
/// ```rust
/// use beancount_importer_rust::utils::time::format_excel_datetime_serial;
///
/// assert_eq!(format_excel_datetime_serial(0.5), "12:00:00");
/// assert_eq!(format_excel_datetime_serial(46087.0), "2026-03-06");
/// assert_eq!(format_excel_datetime_serial(46087.60920138889), "2026-03-06 14:37:15");
/// ```
pub fn format_excel_datetime_serial(serial: f64) -> String {
    if !serial.is_finite() {
        return serial.to_string();
    }

    let epoch = match NaiveDate::from_ymd_opt(1899, 12, 30) {
        Some(date) => date,
        None => return serial.to_string(),
    };

    let days = serial.trunc() as i64;
    let fraction = serial.fract().abs();
    // 与 `parse_excel_serial_time` 保持一致，按秒级稳定化浮点小数部分。
    let mut seconds = (fraction * 86_400.0).round() as i64;
    if seconds >= 86_400 {
        seconds = 0;
    }

    let Some(date) = epoch.checked_add_signed(Duration::days(days)) else {
        return serial.to_string();
    };
    let Some(date_time) = date
        .and_hms_opt(0, 0, 0)
        .and_then(|base| base.checked_add_signed(Duration::seconds(seconds)))
    else {
        return serial.to_string();
    };

    if days == 0 && seconds > 0 {
        return date_time.time().format("%H:%M:%S").to_string();
    }
    if seconds > 0 {
        return date_time.format("%Y-%m-%d %H:%M:%S").to_string();
    }

    date.format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::{format_excel_datetime_serial, normalize_time_text, parse_excel_serial_date};

    #[test]
    fn normalizes_excel_serial_time_to_hhmmss() {
        assert_eq!(normalize_time_text("46110.5"), Some("12:00:00".to_string()));
    }

    #[test]
    fn normalizes_datetime_text_to_hhmmss() {
        assert_eq!(
            normalize_time_text("2026-03-06 14:37:15"),
            Some("14:37:15".to_string())
        );
        assert_eq!(normalize_time_text("14:37"), Some("14:37:00".to_string()));
    }

    #[test]
    fn parses_excel_serial_date_with_reasonable_year_guard() {
        assert_eq!(
            parse_excel_serial_date("46000").map(|date| date.to_string()),
            Some("2025-12-09".to_string())
        );
        assert_eq!(parse_excel_serial_date("100"), None);
    }

    #[test]
    fn formats_excel_datetime_serial_stably() {
        assert_eq!(
            format_excel_datetime_serial(46087.60920138889),
            "2026-03-06 14:37:15"
        );
        assert_eq!(format_excel_datetime_serial(0.5), "12:00:00");
        assert_eq!(format_excel_datetime_serial(46087.0), "2026-03-06");
    }
}
