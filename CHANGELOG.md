# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.3] - 2026-08-31

### Fixed
- seed 扫描跳过 tmp 暂存目录，避免输出副本导致 lot 重复注册（逆回购赎回匹配到错误日期）

## [0.2.2] - 2026-08-31

### Fixed
- 份额分拆新份额成本从已移除旧份额推算，不再依赖数据源缺失的 netPnl 字段（银河证券对账单无此列）

## [0.2.1] - 2026-08-31

### Fixed
- seed 库存按当前批次最早交易日截断，避免扫描到当前批次自身输出文件造成自引用与 FIFO lot 错配
- seed 目录扫描按路径排序，保证跨月 lot 按时间序回放

## [0.2.0] - 2026-07-17

### Added
- `--batch` 批量导入模式，一次跑完当月全部 provider
- 规则引擎 `fields` 数组语法，一条条件匹配多个字段
- 规则引擎正则捕获组替换 `{1}` `{2}`
- extract_fund_product.py 从 budget-tool 迁移，支持 `--prefix` 参数

### Changed
- 架构重构：消除双轨错误、Pipeline 流水线化、Provider 解耦
- 规则分层策略：payment method → category 两层分离
- Mapping 加载回退链：CLI `--mapping` → `mapping_file` → 分层路径 → 内嵌
- `source` metadata 统一使用 `display_name()`

### Docs
- 重写 README + 新增 QUICKSTART.md + CONTRIBUTING.md
- 精简 docs/，1850 行 → 500 行核心文档

## [0.1.0] - 2026-05-07

### Added
- Initial public version.
