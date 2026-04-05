//! 币种类别判定工具。
//!
//! 当前提供“是否法币”的统一判定逻辑，供运行时库存/PnL 与写出层复用，
//! 避免在多个模块维护重复白名单。

/// 判断给定代码是否应视为“法币现金”。
///
/// # 参数
/// - `currency`：币种或商品代码（例如 `CNY`、`USD`、`SEC_159915`）。
///
/// # 返回值
/// - `true`：属于法币现金白名单；
/// - `false`：不在白名单内。
pub fn is_fiat_currency(currency: &str) -> bool {
    matches!(
        currency,
        "CNY" | "USD" | "HKD" | "EUR" | "JPY" | "GBP" | "SGD" | "CHF" | "AUD" | "CAD"
    )
}
