//! 模块说明：跨 Provider 的证券交易分类、账户规划与分录构建能力。
//!
//! 文件路径：src/providers/shared/securities/mod.rs。
//! 该文件主要承担子模块声明与导出职责。
//! 关键符号：无显式公开符号，主要通过内部实现或模块组织发挥作用。

mod context;
mod logic;
mod normalize;
mod posting;
mod trade;
mod trade_accounts;
mod trade_repo;
mod trade_spot;
mod transfer;
mod transform;

pub(crate) use transform::transform_security_record;

/// 证券转换共享参数。
#[derive(Debug, Clone, Copy)]
pub(crate) struct SecurityTransformOptions {
    /// 供应商标识（如 `futu`、`yinhe`），用于 metadata 规范化和来源标签。
    pub(crate) provider_name: &'static str,
    /// 当原始记录缺少交易对手时使用的默认 payee。
    pub(crate) default_payee: &'static str,
}

/// 逆回购统一按每份 100 CNY 面值建模。
pub(super) const REPO_FACE_VALUE: i64 = 100;

/// 银证转账对手资产账户默认值。
pub(super) const DEFAULT_TRANSFER_ASSET_ACCOUNT: &str = "Assets:Transfer:Broker";
