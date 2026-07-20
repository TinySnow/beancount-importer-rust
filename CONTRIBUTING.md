# 供应商扩展指南

接一个新供应商需要改的东西。看完就能上手。

---

## 一、决策树

先判断你属于哪种情况：

```
只是同一个供应商改了 CSV 列名？
  → 只改 YAML（config/*.yml + mapping/*.yml）

新增一个银行 / 第三方支付的供应商？
  → 改 4 个文件 + 2 个注册点（约 5 分钟）

新增一个证券供应商？
  → 改 4 个文件 + 2 个注册点 + securities_accounts 配置（约 10 分钟）
```

---

## 二、新增现金流供应商（银行 / 第三方支付）

以接入 **招商银行 `cmb`** 为例。

### 需要改/建的文件

| # | 文件 | 操作 | 写什么 |
|---|---|---|---|
| 1 | `src/providers/banks/cmb.rs` | 新建 | Rust provider 实现 |
| 2 | `src/providers/banks/mod.rs` | 改 | 注册模块 |
| 3 | `config/banks/cmb.yml` | 新建 | 供应商配置 |
| 4 | `mapping/banks/cmb.yml` | 新建 | 字段映射 |

### Step 1 — Rust provider（`src/providers/banks/cmb.rs`）

```rust
use std::path::Path;

use crate::{
    error::ImporterResult,
    interface::provider::{Provider, parse_tabular_source},
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        mapping::field_mapping::FieldMapping, rule::rule_engine::RuleEngine,
        transaction::Transaction,
    },
    providers::shared::{CashflowTransformOptions, transform_cashflow_record},
};

const CMB_OPTIONS: CashflowTransformOptions = CashflowTransformOptions {
    default_asset_fallback: "Assets:CMB",
};

pub struct CmbProvider;

impl Provider for CmbProvider {
    fn name(&self) -> &'static str {
        "cmb"
    }

    fn description(&self) -> &'static str {
        "CMB statement importer"
    }

    fn display_name(&self) -> &'static str {
        "招商银行"
    }

    fn parse(
        &self, path: &Path, mapping: &FieldMapping,
        config: &ProviderConfig, strict_mode: bool,
    ) -> ImporterResult<Vec<RawRecord>> {
        parse_tabular_source(path, mapping, config, strict_mode)
    }

    fn transform(
        &self, record: RawRecord, rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        transform_cashflow_record(
            self.name(), self.display_name(), CMB_OPTIONS,
            record, rule_engine, config,
        )
    }
}
```

**关键点：**
- `name()` 返回 provider 标识，会被用作 `-p` 参数
- `display_name()` 返回中文名，会写入 `source:` metadata
- `default_asset_fallback` 是所有规则都没命中时的兜底资产账户
- 无需手写任何字段解析、方向推断、分录构建 --- `transform_cashflow_record` 全包了

### Step 2 — 注册（`src/providers/banks/mod.rs`）

加两处：

```rust
pub mod cmb;              // ← 新增行

// ...

pub fn all() -> Vec<Arc<dyn Provider>> {
    vec![
        // ...
        Arc::new(cmb::CmbProvider),   // ← 新增行
    ]
}
```

> **注意：** 第三方支付改 `src/providers/third_party/mod.rs`，证券改 `src/providers/securities/mod.rs`。模式完全一样。

### Step 3 — Provider 配置（`config/banks/cmb.yml`）

```yaml
name: "招商银行"
default:
  asset_account: "Assets:CMB"
  expense_account: "Expenses:Unknown"
  income_account: "Income:Unknown"
  currency: "CNY"
tabular_options:
  delimiter: ","
  flexible: true
  encoding: "auto"
skip_header_lines: 0          # 按实际情况调
has_header_row: true
rules: []
```

**银行类注意：** 如果账单出入账分两列（如工商银行），需要加：

```yaml
tabular_options:
  income_column: "收入金额"
  expense_column: "支出金额"
```

### Step 4 — 字段映射（`mapping/banks/cmb.yml`）

```yaml
date: "交易日期"                # CSV 列名 → 标准字段
payee: "交易对方"
narration: "摘要"
transaction_type: "借贷标志"    # "支出"/"收入" 或 "借"/"贷"
amount:
  column: "交易金额"
  transform: abs
status: ""                     # 没有状态列就留空
reference: "流水号"
date_formats:
  - "%Y%m%d"
  - "%Y-%m-%d"
```

**字段映射速查：**

| 标准字段 | 必填 | 说明 |
|---|---|---|
| `date` | **是** | 交易日期列名 |
| `amount` | **是** | 金额列（支持 `column` / `transform` / `regex_extract`） |
| `payee` | 否 | 交易对方 |
| `narration` | 否 | 摘要/说明 |
| `transaction_type` | 否 | 收/支方向（为空时按金额符号推断） |
| `status` | 否 | 交易状态（用于过滤失败交易） |
| `reference` | 否 | 唯一流水号 |
| `date_formats` | **是** | 按顺序尝试的日期格式 |
| `extra_fields` | 否 | 额外字段（会写入 metadata） |

`amount` 的详细写法：
```yaml
amount:
  column: "金额"
  transform: abs           # abs: 取绝对值  negate: 取反
  default: "0"             # 空值时回退
  regex_extract: "([0-9.]+)"  # 从文本提取数字
```

### 完成后验证

```bash
cargo build --release
./beancount-importer-rust -p cmb -s test.csv \
  -c config/banks/cmb.yml --log-level info
```

---

## 三、新增证券供应商

与现金流供应商的差异只有两处：

### 差异 1 — 用 `transform_security_record` 替代 `transform_cashflow_record`

```rust
use crate::providers::shared::securities::transform::transform_security_record;

fn transform(
    &self, record: RawRecord, rule_engine: &RuleEngine,
    config: &ProviderConfig,
) -> ImporterResult<Option<Transaction>> {
    transform_security_record(
        self.name(), self.display_name(), record, rule_engine, config,
    )
}
```

`transform_security_record` 自动处理：银证转账、普通买卖、逆回购。

### 差异 2 — Provider 配置加 `securities_accounts`

```yaml
name: "新券商"
default:
  asset_account: "Assets:Broker:NewBroker:Securities"
tabular_options:
  delimiter: ","
  flexible: true
  encoding: "auto"
securities_accounts:
  cash_account: "Assets:Broker:NewBroker:Cash"
  fee_account: "Expenses:Broker:NewBroker:Fee"
  pnl_account: "Income:Broker:NewBroker:PnL"
  rounding_account: "Expenses:Broker:NewBroker:Rounding"
  repo_interest_account: "Income:Investing:Interest"
output:
  decimal_places: 4
  emit_open_directives: true
  booking_method: "FIFO"
```

### 证券类账单识别逻辑

无需额外代码，`transform_security_record` 会自动识别：

| 交易类型 | 识别规则 |
|---|---|
| 银证转账 | `peer` 或 `交易类型` 含 `银证转账` |
| 逆回购 | `symbol` 为 `204001`/`131810` 或 `交易类型` 含 `回购` |
| 普通买入 | `交易类型` 含 `买入` / `buy` / `申购` / `认购` |
| 普通卖出 | `交易类型` 含 `卖出` / `sell` / `赎回` |

---

## 四、发布检查清单

新增供应商后，PR 前确认：

- [ ] `cargo build --release` 成功
- [ ] `cargo test --quiet` 通过
- [ ] `cargo fmt --check` 通过
- [ ] 用自己的真实账单跑过（普通模式 + `--strict` 模式）
- [ ] `.rs` 文件放在正确目录（`banks` / `third_party` / `securities`）
- [ ] `mod.rs` 的 `all()` 里已注册
- [ ] 配置使用新格式（`default:` 分组），不用旧平铺键
- [ ] 映射文件覆盖了账单的所有列（至少 `date`、`amount`、`payee`、`narration`）
