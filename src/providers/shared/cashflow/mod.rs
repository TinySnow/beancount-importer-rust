//! 现金流（银行/钱包/第三方支付）Provider 的共享转换模块。
//!
//! 设计目标：
//! - 对外暴露统一入口，Provider 仅保留参数配置。
//! - 将方向判定、分录构建与主流程解耦，便于未来扩展更多现金流业务。

mod classify;
mod posting;
mod transform;

pub(crate) use transform::transform_cashflow_record;

/// 现金流转换共享参数。
#[derive(Debug, Clone, Copy)]
pub(crate) struct CashflowTransformOptions {
    /// 供应商标识（如 `wechat`、`icbc`）。
    pub(crate) provider_name: &'static str,
    /// 当未在规则或配置中指定资产账户时的兜底账户。
    pub(crate) default_asset_fallback: &'static str,
}
