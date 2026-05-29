//! 账户与分录基础模型。
//!
//! 本模块定义会计核心数据结构：
//! - [`amount::Amount`]：金额（数值 + 币种）；
//! - [`cost::Cost`]：成本基础（证券买入成本）；
//! - [`price::Price`]：价格信息；
//! - [`posting::Posting`]：过账项。

pub mod amount;
pub mod cost;
pub mod posting;
pub mod price;
