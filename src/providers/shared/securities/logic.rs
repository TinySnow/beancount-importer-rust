//! 模块说明：跨 Provider 的证券交易分类、账户规划与分录构建能力。
//!
//! 文件路径：src/providers/shared/securities/logic.rs。
//! 该文件围绕 'logic' 的职责提供实现。
//! 关键符号：is_cash_transfer_keyword、derives_rounding_account_from_fee_account_prefix、detects_cash_transfer_without_symbol、infers_transfer_direction_from_type_or_amount。

use rust_decimal::Decimal;

/// 交易种类语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransactionKind {
    /// 纯现金划转（如银证转账）。
    CashTransfer,
    /// 证券交易（买入、卖出、逆回购等）。
    SecurityTrade,
}

/// 资金方向语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Direction {
    /// 资金流入券商现金账户。
    In,
    /// 资金流出券商现金账户。
    Out,
}

/// 证券交易方向语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TradeDirection {
    /// 买入方向。
    Buy,
    /// 卖出方向。
    Sell,
}

/// 从手续费账户推导舍入差异账户。
///
/// 例如：
/// - `Expenses:Broker:Galaxy:Fee` -> `Expenses:Broker:Galaxy:Rounding`
/// - 无层级时回落到 `Expenses:Investing:Rounding`
pub(super) fn derive_rounding_account(fee_account: &str) -> String {
    if let Some((prefix, _)) = fee_account.rsplit_once(':') {
        return format!("{}:Rounding", prefix);
    }

    "Expenses:Investing:Rounding".to_string()
}

/// 识别交易种类。
///
/// 规则：
/// - 证券代码非空时，强制判定为 `SecurityTrade`；
/// - 否则基于交易类型关键词识别是否为 `CashTransfer`；
/// - 未命中时默认 `SecurityTrade`，由后续校验报缺失字段错误。
pub(super) fn classify_transaction_kind(
    transaction_type: Option<&str>,
    symbol: Option<&str>,
) -> TransactionKind {
    if symbol.filter(|value| !value.trim().is_empty()).is_some() {
        return TransactionKind::SecurityTrade;
    }

    if is_cash_transfer_keyword(transaction_type) {
        return TransactionKind::CashTransfer;
    }

    TransactionKind::SecurityTrade
}

/// 判断交易类型是否包含“银证转账”关键词。
fn is_cash_transfer_keyword(transaction_type: Option<&str>) -> bool {
    transaction_type
        .map(|value| {
            value.contains("银行转证券")
                || value.contains("证券转银行")
                || value.contains("银证转账")
                || value.contains("银证转入")
                || value.contains("银证转出")
        })
        .unwrap_or(false)
}

/// 推断现金划转方向。
pub(super) fn infer_transfer_direction(
    transaction_type: Option<&str>,
    amount: Decimal,
) -> Direction {
    if let Some(raw) = transaction_type {
        let normalized = raw.to_ascii_lowercase();

        if raw.contains("银行转证券") || raw.contains("银证转入") || normalized.contains("in")
        {
            return Direction::In;
        }
        if raw.contains("证券转银行") || raw.contains("银证转出") || normalized.contains("out")
        {
            return Direction::Out;
        }
    }

    if amount.is_sign_positive() {
        Direction::In
    } else {
        Direction::Out
    }
}

/// 由证券持仓账户推导同券商现金账户。
///
/// 例如：
/// - `Assets:Broker:Galaxy:Securities` -> `Assets:Broker:Galaxy:Cash`
/// - `Assets:Broker:Futu:Cash` -> 保持不变
pub(super) fn derive_cash_account(default_asset_account: Option<&str>) -> String {
    if let Some(account) = default_asset_account.map(str::trim) {
        if account.ends_with(":Cash") {
            return account.to_string();
        }
        if let Some(prefix) = account.strip_suffix(":Securities") {
            return format!("{}:Cash", prefix);
        }
    }

    "Assets:Broker:Cash".to_string()
}

/// 推断证券交易方向。
///
/// 先按交易类型关键词判断；若无法判断，则回退到金额符号。
pub(super) fn infer_trade_direction(
    transaction_type: Option<&str>,
    amount: Option<Decimal>,
) -> TradeDirection {
    if let Some(raw) = transaction_type {
        let normalized = raw.to_ascii_lowercase();

        if normalized.contains("sell")
            || raw.contains("卖")
            || raw.contains("赎回")
            || raw.contains("购回")
        {
            return TradeDirection::Sell;
        }

        if normalized.contains("buy")
            || raw.contains("买")
            || raw.contains("申购")
            || raw.contains("回购")
        {
            return TradeDirection::Buy;
        }
    }

    if amount.map(|value| value.is_sign_negative()).unwrap_or(true) {
        TradeDirection::Buy
    } else {
        TradeDirection::Sell
    }
}

/// 判断是否属于逆回购交易。
///
/// 通过常见逆回购代码和交易类型关键词双重识别。
pub(super) fn is_repo_trade(symbol: &str, transaction_type: Option<&str>) -> bool {
    if symbol == "204001" || symbol == "131810" {
        return true;
    }

    transaction_type
        .map(|value| {
            value.contains("逆回购") || value.contains("融券回购") || value.contains("融券购回")
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::{
        Direction, TradeDirection, TransactionKind, classify_transaction_kind, derive_cash_account,
        derive_rounding_account, infer_trade_direction, infer_transfer_direction,
    };

    #[test]
    fn derives_rounding_account_from_fee_account_prefix() {
        assert_eq!(
            derive_rounding_account("Expenses:Investing:Fees"),
            "Expenses:Investing:Rounding"
        );
    }

    #[test]
    fn detects_cash_transfer_without_symbol() {
        assert_eq!(
            classify_transaction_kind(Some("银行转证券"), None),
            TransactionKind::CashTransfer
        );
        assert_eq!(
            classify_transaction_kind(Some("证券转银行"), Some("")),
            TransactionKind::CashTransfer
        );
        assert_eq!(
            classify_transaction_kind(Some("证券买入"), None),
            TransactionKind::SecurityTrade
        );
        assert_eq!(
            classify_transaction_kind(Some("证券卖出"), Some("159915")),
            TransactionKind::SecurityTrade
        );
    }

    #[test]
    fn infers_transfer_direction_from_type_or_amount() {
        assert_eq!(
            infer_transfer_direction(Some("银行转证券"), Decimal::new(5000, 0)),
            Direction::In
        );
        assert_eq!(
            infer_transfer_direction(Some("证券转银行"), Decimal::new(-5000, 0)),
            Direction::Out
        );
        assert_eq!(
            infer_transfer_direction(None, Decimal::new(1, 0)),
            Direction::In
        );
        assert_eq!(
            infer_transfer_direction(None, Decimal::new(-1, 0)),
            Direction::Out
        );
    }

    #[test]
    fn derives_cash_account_from_securities_account() {
        assert_eq!(
            derive_cash_account(Some("Assets:Broker:Galaxy:Securities")),
            "Assets:Broker:Galaxy:Cash"
        );
        assert_eq!(
            derive_cash_account(Some("Assets:Broker:Futu:Cash")),
            "Assets:Broker:Futu:Cash"
        );
    }

    #[test]
    fn infers_trade_direction_from_type_or_amount() {
        assert_eq!(
            infer_trade_direction(Some("证券买入"), Some(Decimal::new(-100, 0))),
            TradeDirection::Buy
        );
        assert_eq!(
            infer_trade_direction(Some("证券卖出"), Some(Decimal::new(100, 0))),
            TradeDirection::Sell
        );
        assert_eq!(
            infer_trade_direction(None, Some(Decimal::new(-1, 0))),
            TradeDirection::Buy
        );
        assert_eq!(
            infer_trade_direction(None, Some(Decimal::new(1, 0))),
            TradeDirection::Sell
        );
    }
}
