//! 跨 Provider 的现金流转换模块。
//!
//! 关注“现金进出”场景（例如银行卡流水、钱包账单、第三方支付明细），
//! 通过统一流程完成：
//! - 收支方向推断；
//! - 借贷账户选择；
//! - 双分录构建；
//! - 规则输出与扩展元数据附加。

mod classify;
mod posting;
mod transform;

pub(crate) use transform::transform_cashflow_record;

/// 现金流转换共享参数。
///
/// 由各 Provider 在调用共享转换入口时传入，用于绑定 Provider 语义。
#[derive(Debug, Clone, Copy)]
pub(crate) struct CashflowTransformOptions {
    /// 供应商标识（如 `wechat`、`icbc`）。
    pub(crate) provider_name: &'static str,
    /// 当未在规则或配置中指定资产账户时的兜底账户。
    pub(crate) default_asset_fallback: &'static str,
}
