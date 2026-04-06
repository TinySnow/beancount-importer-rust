//! 通用默认值配置（`default:` 分组）。
//!
//! 该结构用于承载 YAML 中的：
//!
//! ```yaml
//! default:
//!   asset_account: "Assets:..."
//!   expense_account: "Expenses:..."
//!   income_account: "Income:..."
//!   currency: "CNY"
//! ```
//!
//! 该分组仅接受新格式字段，不再兼容历史平铺字段命名。

use serde::{Deserialize, Serialize};

/// 通用默认字段分组。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CommonDefaultsConfig {
    /// 默认资产账户。
    #[serde(default)]
    pub asset_account: Option<String>,

    /// 默认支出账户。
    #[serde(default)]
    pub expense_account: Option<String>,

    /// 默认收入账户。
    #[serde(default)]
    pub income_account: Option<String>,

    /// 默认币种。
    #[serde(default)]
    pub currency: Option<String>,
}
