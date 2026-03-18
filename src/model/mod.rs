//! 模块说明：核心领域模型总入口。
//!
//! 文件路径：src/model/mod.rs。
//! 该文件主要承担子模块声明与导出职责。
//! 关键符号：account、cli、config、data。

pub mod account;
pub mod cli;
pub mod config;
pub mod data;
pub mod mapping;
pub mod reader;
pub mod registry;
pub mod rule;
pub mod transaction;
pub mod writer;
