//! 模块说明：规则匹配、条件运算与动作执行引擎。
//!
//! 文件路径：src/model/rule/rule_action.rs。
//! 该文件围绕 'rule_action' 的职责提供实现。
//! 关键符号：RuleAction。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 规则匹配后的操作
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleAction {
    /// 设置借方账户（费用/资产增加）
    pub debit_account: Option<String>,

    /// 设置贷方账户（资产减少/收入）
    pub credit_account: Option<String>,

    /// 设置手续费账户（覆盖默认手续费账户）
    pub fee_account: Option<String>,

    /// 设置已实现损益账户（覆盖默认损益账户）
    pub pnl_account: Option<String>,

    /// 设置尾差账户（覆盖默认尾差账户）
    pub rounding_account: Option<String>,

    /// 设置交易对手
    pub payee: Option<String>,

    /// 设置/追加描述
    pub narration: Option<String>,

    /// 添加标签
    #[serde(default)]
    pub tags: Vec<String>,

    /// 添加链接
    #[serde(default)]
    pub links: Vec<String>,

    /// 设置交易标记
    pub flag: Option<char>,

    /// 设置元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,

    /// 是否忽略此交易
    #[serde(default)]
    pub ignore: bool,
}
