//! 文本辅助工具。
//!
//! 当前提供基础 ASCII 前缀判断，用于校验币种代码等标识符。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::utils::text::starts_with_ascii_letter;
//!
//! assert!(starts_with_ascii_letter("USD"));
//! assert!(!starts_with_ascii_letter("1USD"));
//! ```

/// 判断字符串首字符是否为 ASCII 字母。
///
/// 空字符串返回 `false`。
///
/// # 示例
/// ```rust
/// use beancount_importer_rust::utils::text::starts_with_ascii_letter;
///
/// assert!(starts_with_ascii_letter("A1"));
/// assert!(!starts_with_ascii_letter("_A1"));
/// assert!(!starts_with_ascii_letter(""));
/// ```
pub fn starts_with_ascii_letter(value: &str) -> bool {
    value
        .chars()
        .next()
        .map(|ch| ch.is_ascii_alphabetic())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::starts_with_ascii_letter;

    #[test]
    fn detects_ascii_letter_prefix() {
        assert!(starts_with_ascii_letter("USD"));
        assert!(starts_with_ascii_letter("a1"));
        assert!(!starts_with_ascii_letter("1A"));
        assert!(!starts_with_ascii_letter(""));
    }
}
