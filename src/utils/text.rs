//! 模块说明：通用文本判断工具。
//!
//! 文件路径：src/utils/text.rs。
//! 该文件围绕 ASCII 字符串判断职责提供实现。
//! 关键符号：starts_with_ascii_letter。

/// 判断字符串首字符是否为 ASCII 字母。
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
