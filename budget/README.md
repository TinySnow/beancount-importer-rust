# 预算旁线配置

本目录是“预算分配系统”的配置层，不会影响 importer 主流程。

## 文件说明

- `budgets.yaml`：预算额度（支持 `YYYY-MM` 与 `YYYY-MM 标签`）
- `mappings.yaml`：预算桶映射与预算桶类型配置

## `budgets.yaml` 规则

- `YYYY-MM`：当月基础预算。
- `YYYY-MM 标签`：同月额外预算（例如 `2026-06 绩效`、`2026-06 年终奖`）。
- 同月多个 key 会自动聚合到该月预算统计。

## `mappings.yaml` 规则

- `default_expense_bucket`：未显式标注预算桶的 `Expenses:*` 默认归属（建议设为 `生活费`）。
- `defaults`：账户前缀到预算桶映射（最长前缀优先）。
- `bucket_types`：预算桶类型，`expense`（消费类）或 `asset`（资产类，如储蓄）。
- `asset_bucket_accounts`（可选）：资产类预算桶的账户前缀列表，用于精确定位“钱在哪”。

## 使用方式

### 1. 月度总览（仅目标月）

```bash
cargo run --bin budget_report -- \
  --ledger /path/to/ledger.beancount \
  --month 2026-06 \
  --budgets budget/budgets.yaml \
  --mappings budget/mappings.yaml \
  --scope month
```

### 2. 累计总览（截至目标月）

```bash
cargo run --bin budget_report -- \
  --ledger /path/to/ledger.beancount \
  --month 2026-06 \
  --scope cumulative
```

### 3. 查看某个预算桶历史（汇总/分月/明细）

```bash
# 汇总
cargo run --bin budget_report -- \
  --ledger /path/to/ledger.beancount \
  --month 2026-06 \
  --bucket 旅行 \
  --scope cumulative \
  --bucket-view summary

# 分月
cargo run --bin budget_report -- \
  --ledger /path/to/ledger.beancount \
  --month 2026-06 \
  --bucket 生活费 \
  --scope month \
  --bucket-view monthly

# 明细
cargo run --bin budget_report -- \
  --ledger /path/to/ledger.beancount \
  --month 2026-06 \
  --bucket 生活费 \
  --scope month \
  --bucket-view detail
```

### 4. 查看资产类预算桶“钱在哪”

```bash
cargo run --bin budget_report -- \
  --ledger /path/to/ledger.beancount \
  --month 2026-06 \
  --bucket 储蓄 \
  --scope cumulative \
  --bucket-view detail \
  --show-locations
```

## 预算归属优先级

1. 交易 metadata：`budget: "预算桶名"`（兼容历史误拼写 `budge`）。
2. `mappings.yaml` 中 `defaults` 的账户前缀映射（最长前缀优先）。
3. 若仍未命中且为 `Expenses:*`，自动归入 `default_expense_bucket`（默认 `生活费`）。

