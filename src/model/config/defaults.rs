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
//! 兼容说明：
//! - 分组内也接受历史命名（如 `default_asset_account`），便于平滑迁移。

use serde::{Deserialize, Serialize};

/// 通用默认字段分组。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommonDefaultsConfig {
    /// 默认资产账户。
    #[serde(default, alias = "default_asset_account")]
    pub asset_account: Option<String>,

    /// 默认支出账户。
    #[serde(default, alias = "default_expense_account")]
    pub expense_account: Option<String>,

    /// 默认收入账户。
    #[serde(default, alias = "default_income_account")]
    pub income_account: Option<String>,

    /// 默认币种。
    #[serde(default, alias = "default_currency")]
    pub currency: Option<String>,
}
