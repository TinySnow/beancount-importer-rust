//! 接口层模块。
//!
//! `interface` 用于定义系统边界上的抽象契约，而不关心具体供应商实现细节。
//! 当前主要暴露 [`provider`] 模块中的 [`Provider`](provider::Provider) trait，
//! 供 `src/providers/` 下的适配器实现。
//!
//! # 设计目标
//! - 统一不同来源账单的解析入口。
//! - 将“原始读取”与“业务转换”拆分为可组合阶段。
//! - 让运行时流程依赖抽象接口，降低新增供应商时的耦合成本。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::interface::provider::Provider;
//!
//! fn accepts_provider(_provider: &dyn Provider) {}
//! ```

pub mod provider;
