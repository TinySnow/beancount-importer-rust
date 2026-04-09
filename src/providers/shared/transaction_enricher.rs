//! 交易补充与元数据写入工具。
//!
//! 该模块负责把规则引擎输出、Provider 补充字段以及来源标签
//! 统一写入 `Transaction`，避免各转换流程重复实现同一套元数据逻辑。

use crate::model::{
    config::meta_value::MetaValue, rule::match_result::MatchResult, transaction::Transaction,
};
use crate::utils::metadata::normalize_metadata_key;

/// 将规则匹配结果附加到交易对象。
///
/// 会按统一顺序写入：
/// 1. 可覆盖字段（`payee`、`flag`）；
/// 2. 集合字段（`tags`、`links`）；
/// 3. 规则元数据（经过 provider 前缀归一化）；
/// 4. `source` 来源标签。
///
/// `fallback_payee` 在规则未命中 `payee` 时生效，通常用于保留原始记录中的对手方。
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

/// 将原始记录中的扩展字段写入交易元数据。
///
/// 元数据键会先经过 [`normalize_metadata_key`] 处理，确保不同 Provider
/// 的同名字段不会发生命名冲突。
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

/// 按规范化键名写入 `orderId` 元数据。
///
/// 当 `order_id` 为空时不写入任何字段。
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

/// 解析交易来源标签（`source`）。
///
/// 优先使用 `provider_display_name`，其次使用 `provider_name`。
fn resolve_provider_source(provider_name: &str, provider_display_name: Option<&str>) -> String {
    provider_display_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(provider_name)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::resolve_provider_source;

    #[test]
    fn falls_back_to_provider_name_when_display_name_is_absent() {
        assert_eq!(resolve_provider_source("wechat", None), "wechat");
    }

    #[test]
    fn prefers_display_name_when_present() {
        assert_eq!(
            resolve_provider_source("wechat", Some("自定义来源")),
            "自定义来源"
        );
    }
}
