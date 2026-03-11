//! 证券 Provider 的共享转换模块。
//!
//! 设计目标：
//! - 将“券商通用账务语义”沉淀在共享层，Provider 仅保留名称与少量参数。
//! - 按职责拆分文件，避免单文件过长导致维护困难。
//! - 通过场景分支模块化，为后续股票等更多证券业务扩展预留空间。

mod logic;
mod normalize;
mod posting;
mod trade;
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
