//! 跨 Provider 的证券交易转换模块。
//!
//! 统一处理证券相关流水的三类核心场景：
//! - 普通买卖（现货）；
//! - 逆回购；
//! - 银证资金划转。
//!
//! 模块内部完成交易分类、账户规划、分录构建和元数据补充，
//! 对外暴露单一入口 `transform_security_record`。

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
///
/// 每个证券类 Provider 在调用共享转换入口时提供该配置。
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
