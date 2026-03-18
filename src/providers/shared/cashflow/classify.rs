//! 模块说明：跨 Provider 的现金流分类与分录构建能力。
//!
//! 文件路径：src/providers/shared/cashflow/classify.rs。
//! 该文件围绕 'classify' 的职责提供实现。
//! 关键符号：infers_expense_from_direction_keywords、infers_expense_from_english_keywords、falls_back_to_amount_sign_when_direction_missing。

use rust_decimal::Decimal;

/// 根据方向字段与金额符号推断是否为支出。
///
/// 判定优先级：
/// 1. 方向字段中的中英文关键词。
/// 2. 若方向缺失或无法识别，则回退到金额符号（正数视为支出）。
pub(super) fn infer_is_expense(direction: Option<&str>, amount: Decimal) -> bool {
    if let Some(raw) = direction {
        let normalized = raw.to_ascii_lowercase();
        if raw.contains("支出") || raw.contains("转出") {
            return true;
        }
        if raw.contains("收入") || raw.contains("转入") {
            return false;
        }
        if normalized.contains("expense") || normalized.contains("out") {
            return true;
        }
        if normalized.contains("income") || normalized.contains("in") {
            return false;
        }
    }

    amount > Decimal::ZERO
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::infer_is_expense;

    #[test]
    fn infers_expense_from_direction_keywords() {
        assert!(infer_is_expense(Some("支出"), Decimal::ZERO));
        assert!(infer_is_expense(Some("转出"), Decimal::ZERO));
        assert!(!infer_is_expense(Some("收入"), Decimal::ZERO));
        assert!(!infer_is_expense(Some("转入"), Decimal::ZERO));
    }

    #[test]
    fn infers_expense_from_english_keywords() {
        assert!(infer_is_expense(Some("expense"), Decimal::ZERO));
        assert!(infer_is_expense(Some("out"), Decimal::ZERO));
        assert!(!infer_is_expense(Some("income"), Decimal::ZERO));
        assert!(!infer_is_expense(Some("in"), Decimal::ZERO));
    }

    #[test]
    fn falls_back_to_amount_sign_when_direction_missing() {
        assert!(infer_is_expense(None, Decimal::new(10, 0)));
        assert!(!infer_is_expense(None, Decimal::new(-10, 0)));
    }
}
