# 预算旁线配置

本目录是“预算分配系统”的配置层，不会影响 importer 主流程。

## 文件说明

- `budgets.yaml`：按月预算额度（`YYYY-MM -> bucket -> amount`）
- `mappings.yaml`：默认预算桶映射（`account_prefix -> bucket`）

## 使用方式

```bash
cargo run --bin budget_report -- \
  --ledger /path/to/ledger.beancount \
  --month 2026-05 \
  --budgets budget/budgets.yaml \
  --mappings budget/mappings.yaml
```

## 预算归属优先级

1. 交易 metadata：`budget: "bucket_name"`（或兼容 `budge`）
2. `mappings.yaml` 中账户前缀映射（最长前缀优先）

只统计 `Expenses:*` 且金额为正的过账。
