//! 模块说明：通用工具函数集合。
//!
//! 文件路径：src/utils/metadata.rs。
//! 该文件围绕 'metadata' 的职责提供实现。
//! 关键符号：normalize_metadata_key、ensure_beancount_metadata_key、map_key、to_lower_camel_ascii。

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 将供应商元数据键规范化为便于迁移的英文键名。
///
/// 主要目标：与 double-entry-generator 风格键名保持兼容。
pub fn normalize_metadata_key(provider: &str, raw_key: &str) -> String {
    let key = raw_key.trim();
    if key.is_empty() {
        return "meta".to_string();
    }

    if let Some(mapped) = map_key(provider, key) {
        return mapped.to_string();
    }

    ensure_beancount_metadata_key(key)
}

/// 确保元数据键是 Beancount 可接受的 ASCII 标识符。
///
/// 若键无法规范化为可用标识符，则返回稳定的
/// 哈希键，格式为 `meta_xxxxxxxx`。
pub fn ensure_beancount_metadata_key(raw_key: &str) -> String {
    let raw = raw_key.trim();
    if raw.is_empty() {
        return "meta".to_string();
    }

    if let Some(converted) = to_lower_camel_ascii(raw) {
        return converted;
    }

    let mut hasher = DefaultHasher::new();
    raw.hash(&mut hasher);
    format!("meta_{:08x}", hasher.finish() as u32)
}

fn map_key(provider: &str, raw_key: &str) -> Option<&'static str> {
    let provider_is_alipay = provider.eq_ignore_ascii_case("alipay");

    match raw_key {
        "交易状态" | "状态" => return Some("status"),
        "来源" | "交易来源" => return Some("source"),
        "交易分类" | "分类" => {
            return Some(if provider_is_alipay {
                "category"
            } else {
                "txType"
            });
        }
        "收/支" | "收支" => return Some("type"),
        "交易时间" | "支付时间" | "付款时间" => return Some("payTime"),
        "收/付款方式" | "支付方式" | "付款方式" | "收款方式" => {
            return Some("method");
        }
        "交易订单号" | "订单号" | "交易流水号" | "流水号" => {
            return Some("orderId");
        }
        "商家订单号" => return Some("merchantId"),
        "交易对方" | "交易对手" | "对方" => return Some("peer"),
        "对方账号" | "对手账户" => return Some("peerAccount"),
        "商品说明" => return Some("item"),
        "备注" => return Some("note"),
        "金额" | "金额(元)" | "金额（元）" => return Some("amount"),
        "币种" => return Some("currency"),
        _ => {}
    }

    let lower = raw_key.to_ascii_lowercase();
    match lower.as_str() {
        "status" | "transactionstatus" | "transaction_status" | "trade_status" => Some("status"),
        "source" | "tradesource" | "trade_source" => Some("source"),
        "txtype"
        | "tx_type"
        | "transactiontype"
        | "transaction_type"
        | "transactioncategory"
        | "transaction_category"
        | "category" => Some(if provider_is_alipay {
            "category"
        } else {
            "txType"
        }),
        "type" | "inout" | "in_out" | "incomeexpense" | "income_expense" | "direction" => {
            Some("type")
        }
        "paytime" | "pay_time" | "transactiontime" | "transaction_time" | "tradetime"
        | "trade_time" | "paymenttime" | "payment_time" => Some("payTime"),
        "method" | "paymethod" | "pay_method" | "paymentmethod" | "payment_method" => {
            Some("method")
        }
        "orderid"
        | "order_id"
        | "transactionorderid"
        | "transaction_order_id"
        | "reference"
        | "transactionid"
        | "transaction_id" => Some("orderId"),
        "merchantid" | "merchant_id" | "merchantorderid" | "merchant_order_id"
        | "merchantorderno" | "merchant_order_no" => Some("merchantId"),
        "peer" | "payee" => Some("peer"),
        "peeraccount" | "peer_account" | "payeeaccount" | "payee_account" => Some("peerAccount"),
        "remark" | "memo" | "comment" | "note" => Some("note"),
        "amount" => Some("amount"),
        "currency" => Some("currency"),
        _ => None,
    }
}

fn to_lower_camel_ascii(raw: &str) -> Option<String> {
    if !raw.is_ascii() {
        return None;
    }

    let mut output = String::new();
    let mut make_upper = false;

    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            let normalized = if output.is_empty() {
                ch.to_ascii_lowercase()
            } else if make_upper {
                make_upper = false;
                ch.to_ascii_uppercase()
            } else {
                ch
            };
            output.push(normalized);
        } else if !output.is_empty() {
            make_upper = true;
        }
    }

    if output.is_empty() {
        return None;
    }

    if output.as_bytes()[0].is_ascii_digit() {
        output.insert(0, '_');
    }

    Some(output)
}

#[cfg(test)]
mod tests {
    use super::{ensure_beancount_metadata_key, normalize_metadata_key};

    #[test]
    fn maps_required_chinese_metadata_keys() {
        assert_eq!(normalize_metadata_key("alipay", "交易状态"), "status");
        assert_eq!(normalize_metadata_key("alipay", "来源"), "source");
        assert_eq!(normalize_metadata_key("alipay", "收/支"), "type");
        assert_eq!(normalize_metadata_key("alipay", "交易时间"), "payTime");
        assert_eq!(normalize_metadata_key("alipay", "交易订单号"), "orderId");
        assert_eq!(normalize_metadata_key("alipay", "商家订单号"), "merchantId");
    }

    #[test]
    fn maps_category_based_on_provider() {
        assert_eq!(normalize_metadata_key("alipay", "交易分类"), "category");
        assert_eq!(normalize_metadata_key("futu", "交易分类"), "txType");
    }

    #[test]
    fn peer_key_mapping_is_strict() {
        assert_eq!(normalize_metadata_key("wechat", "交易对方"), "peer");
        assert_eq!(normalize_metadata_key("wechat", "对方账号"), "peerAccount");

        assert_eq!(
            normalize_metadata_key("wechat", "counterparty"),
            "counterparty"
        );
        assert_eq!(
            normalize_metadata_key("wechat", "counterpartyAccount"),
            "counterpartyAccount"
        );
    }

    #[test]
    fn fallback_keeps_ascii_identifier_valid() {
        assert_eq!(
            ensure_beancount_metadata_key("payment_method"),
            "paymentMethod"
        );
        assert!(ensure_beancount_metadata_key("未知字段").starts_with("meta_"));
    }
}
