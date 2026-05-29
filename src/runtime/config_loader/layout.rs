//! 供应商配置/映射分层目录布局常量。
//!
//! 定义配置加载和映射加载时的候选目录顺序。

/// 供应商配置/映射分层目录名（按尝试顺序）。
pub(super) const PROVIDER_CATEGORY_DIRS: [&str; 3] = ["third_party", "banks", "securities"];
