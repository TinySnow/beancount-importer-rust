//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
//! 日期解析

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use log::trace;

/// 解析日期时间字符串。
///
/// 支持示例：
/// - `2023/12/31 3:44:00`
/// - `2023-12-31 13:44:00`
/// - `2023/12/31`
/// - `2023-12-31`
pub fn parse_datetime(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let parts: Vec<&str> = s.split_whitespace().collect();

    let date = parse_date_part(parts.first()?)?;
    let time = parts
        .get(1)
        .and_then(|s: &&str| parse_time_part(s))
        .unwrap_or(NaiveTime::MIN);

    Some(NaiveDateTime::new(date, time))
}

/// 仅解析日期部分。
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    parse_datetime(s).map(|dt| dt.date())
}

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
