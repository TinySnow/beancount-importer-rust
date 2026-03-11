use rust_decimal::Decimal;

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

/// 判断当前记录是否为“银证转账”类纯现金划转。
///
/// 规则：
/// - 若有非空证券代码，则视为证券买卖而非现金划转。
/// - 否则根据业务类型中的关键字识别。
pub(super) fn is_cash_transfer_record(
    transaction_type: Option<&str>,
    symbol: Option<&str>,
) -> bool {
    if symbol.filter(|value| !value.trim().is_empty()).is_some() {
        return false;
    }

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
///
/// 返回值：
/// - `true`：资金流入券商账户。
/// - `false`：资金流出券商账户。
pub(super) fn infer_transfer_in(transaction_type: Option<&str>, amount: Decimal) -> bool {
    if let Some(raw) = transaction_type {
        let normalized = raw.to_ascii_lowercase();

        if raw.contains("银行转证券") || raw.contains("银证转入") || normalized.contains("in")
        {
            return true;
        }
        if raw.contains("证券转银行") || raw.contains("银证转出") || normalized.contains("out")
        {
            return false;
        }
    }

    amount.is_sign_positive()
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

/// 推断是否为买入方向。
///
/// 先按业务类型关键字判断；若无法判断，则回退到金额符号。
pub(super) fn infer_is_buy(transaction_type: Option<&str>, amount: Option<Decimal>) -> bool {
    if let Some(raw) = transaction_type {
        let normalized = raw.to_ascii_lowercase();

        if normalized.contains("sell")
            || raw.contains("卖")
            || raw.contains("赎回")
            || raw.contains("购回")
        {
            return false;
        }

        if normalized.contains("buy")
            || raw.contains("买")
            || raw.contains("申购")
            || raw.contains("回购")
        {
            return true;
        }
    }

    amount.map(|value| value.is_sign_negative()).unwrap_or(true)
}

/// 判断是否属于逆回购交易。
///
/// 通过常见逆回购代码和业务类型关键字双重识别。
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
        derive_cash_account, derive_rounding_account, infer_transfer_in, is_cash_transfer_record,
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
        assert!(is_cash_transfer_record(Some("银行转证券"), None));
        assert!(is_cash_transfer_record(Some("证券转银行"), Some("")));
        assert!(!is_cash_transfer_record(Some("证券买入"), None));
        assert!(!is_cash_transfer_record(Some("证券卖出"), Some("159915")));
    }

    #[test]
    fn infers_transfer_direction_from_type_or_amount() {
        assert!(infer_transfer_in(Some("银行转证券"), Decimal::new(5000, 0)));
        assert!(!infer_transfer_in(
            Some("证券转银行"),
            Decimal::new(-5000, 0)
        ));
        assert!(infer_transfer_in(None, Decimal::new(1, 0)));
        assert!(!infer_transfer_in(None, Decimal::new(-1, 0)));
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
}
