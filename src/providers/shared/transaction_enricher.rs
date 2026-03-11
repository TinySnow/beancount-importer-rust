//! Provider 层交易补充与元数据处理工具。

use crate::model::{
    config::meta_value::MetaValue, rule::match_result::MatchResult, transaction::Transaction,
};
use crate::utils::metadata::normalize_metadata_key;

/// 应用规则匹配得到的通用交易属性。
pub(crate) fn apply_match_result(
    mut tx: Transaction,
    provider_name: &str,
    match_result: &MatchResult,
    fallback_payee: Option<String>,
    provider_display_name: Option<&str>,
) -> Transaction {
    if let Some(payee) = match_result.payee.clone().or(fallback_payee) {
        tx = tx.with_payee(payee);
    }

    if let Some(flag) = match_result.flag {
        tx = tx.with_flag(flag);
    }

    for tag in &match_result.tags {
        tx = tx.with_tag(tag.clone());
    }

    for link in &match_result.links {
        tx = tx.with_link(link.clone());
    }

    for (key, value) in &match_result.metadata {
        let normalized_key = normalize_metadata_key(provider_name, key);
        tx = tx.with_meta(normalized_key, MetaValue::String(value.clone()));
    }

    tx = tx.with_meta(
        "source",
        MetaValue::String(resolve_provider_source(
            provider_name,
            provider_display_name,
        )),
    );

    tx
}

/// 将扩展字段按供应商规范附加为元数据。
pub(crate) fn append_extra_metadata<I>(
    mut tx: Transaction,
    provider_name: &str,
    extra_fields: I,
) -> Transaction
where
    I: IntoIterator<Item = (String, String)>,
{
    for (key, value) in extra_fields {
        let normalized_key = normalize_metadata_key(provider_name, &key);
        tx = tx.with_meta(normalized_key, MetaValue::String(value));
    }

    tx
}

/// 使用规范化键名附加订单号元数据。
pub(crate) fn append_order_id(
    mut tx: Transaction,
    provider_name: &str,
    order_id: Option<String>,
) -> Transaction {
    if let Some(order_id) = order_id {
        let key = normalize_metadata_key(provider_name, "orderId");
        tx = tx.with_meta(key, MetaValue::String(order_id));
    }

    tx
}

/// 解析来源标签（`source`），优先使用供应商配置显示名。
fn resolve_provider_source(provider_name: &str, provider_display_name: Option<&str>) -> String {
    let hinted = provider_display_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(provider_name);

    map_provider_source(hinted)
        .or_else(|| map_provider_source(provider_name))
        .unwrap_or_else(|| hinted.to_string())
}

/// 将常见供应商名称映射为中文来源标签。
fn map_provider_source(raw: &str) -> Option<String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "wechat" | "weixin" => Some("微信".to_string()),
        "alipay" => Some("支付宝".to_string()),
        "icbc" => Some("工商银行".to_string()),
        "ccb" => Some("建设银行".to_string()),
        "jd" | "jingdong" => Some("京东".to_string()),
        "mt" | "meituan" => Some("美团".to_string()),
        "yinhe" | "galaxy" => Some("银河证券".to_string()),
        "futu" => Some("富途".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_provider_source;

    #[test]
    fn resolves_known_provider_source_label() {
        assert_eq!(resolve_provider_source("wechat", None), "微信");
        assert_eq!(resolve_provider_source("futu", Some("yinhe")), "银河证券");
    }

    #[test]
    fn falls_back_to_display_name_for_unknown_provider() {
        assert_eq!(
            resolve_provider_source("custom", Some("自定义来源")),
            "自定义来源"
        );
    }
}
