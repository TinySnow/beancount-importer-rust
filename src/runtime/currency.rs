//! 模块说明：币种标准化辅助能力。
//!
//! 文件路径：src/runtime/currency.rs。
//! 该文件围绕 'currency' 的职责提供实现。
//! 关键符号：无显式公开符号，主要通过内部实现或模块组织发挥作用。

/// 判断给定币种是否应视为法币现金。
///
/// 该分类会被 lot 解析与 PnL 计算复用，用于区分“证券持仓分录”和“现金分录”。
pub(crate) fn is_fiat_currency(currency: &str) -> bool {
    matches!(
        currency,
        "CNY" | "USD" | "HKD" | "EUR" | "JPY" | "GBP" | "SGD" | "CHF" | "AUD" | "CAD"
    )
}
