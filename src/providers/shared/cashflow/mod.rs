//! 模块说明：跨 Provider 的现金流分类与分录构建能力。
//!
//! 文件路径：src/providers/shared/cashflow/mod.rs。
//! 该文件主要承担子模块声明与导出职责。
//! 关键符号：无显式公开符号，主要通过内部实现或模块组织发挥作用。

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
