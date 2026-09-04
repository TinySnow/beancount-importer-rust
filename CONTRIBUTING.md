# CONTRIBUTING

开发者手册：架构设计、开发调试、发布流程、新增供应商。

## 1. 架构设计

### 1.1 总体流程

项目采用"读取 → 归一化 → 规则匹配 → 分录生成 → Beancount 输出（输出前经过排序/库存补全/PnL 三道后处理）"的流水线：

```text
账单文件(CSV/XLSX)
  -> TabularRecordReader
  -> RawRecord
  -> RuleEngine(全局规则 + provider 规则)
  -> Provider::transform
  -> Transaction
  -> 稳定排序 (sorting)
  -> 卖出 lot 成本补全 (inventory)
  -> 逐笔收益标注 (pnl)
  -> BeancountWriter
  -> .beancount
```

### 1.2 模块分层

- **`runtime`**：入口编排与 I/O。`config_loader/*` 加载合并配置、`cli/*` 参数模型、`reader/tabular/*` CSV/XLSX 读取、`writer/beancount_writer/*` 文本渲染、`provider_registry.rs` provider 注册表、`pipeline.rs` 可插拔流水线（默认：排序→库存补全→PnL 标注；银行/三方用 `cashflow_only()` 仅排序）、`sorting.rs` 稳定排序、`inventory/*` lot 补全、`pnl.rs` 逐笔收益元数据。
- **`interface`**：`Provider` trait（`name/description/display_name/parse/transform`）+ `parse_tabular_source` 辅助函数。
- **`model`**：纯领域层。`data/raw_record.rs` 中间记录、`rule/*` 规则、`transaction/*` 交易模型、`account/*` `config/*` `mapping/*` 数据结构。
- **`providers`**：`third_party/*` `banks/*` `securities/*` 各供应商；`shared/cashflow/*` 通用收支模型、`shared/securities/*` 通用证券模型、`shared/transaction_enricher.rs` 元数据写入。
- **`utils`**：编码识别、金额解析、metadata key 归一化。

### 1.3 规则引擎

- 执行顺序：`global_rules` → provider `rules` → 各自 `priority` 升序 → 条件数升序 → 文件顺序。
- 结果合并：单值字段后命中覆盖；`metadata` 按 key 合并；`tags/links` 追加；`ignore` 一旦 true 持续 true。
- 行为控制：`terminal: true` 命中即停止；`ignore: true` 不输出。

### 1.4 现金流与证券两条转换链

- **现金流链**（bank / third-party）：优先看 `transaction_type` 关键词判方向，回退金额符号。支出借费用贷资产，收入借资产贷收入。
- **证券链**（securities）：普通买入/卖出、逆回购（固定面值 100 CNY/份）、银证转账（仅资产划转，不走 pnl/rounding）。

### 1.5 输出层

- 交易按日期排序输出，结果稳定。
- 自动写出 `commodity`（非法币且涉及持仓/价格时）。
- 可选写出 `open`（`emit_open_directives=true`）。
- metadata 按 key 排序，输出可比较。

## 2. 开发与调试

### 2.1 建议开发流程

1. 修改代码或配置。
2. `cargo fmt --check`
3. `cargo test --quiet`
4. 用真实或脱敏样本跑一次 `cargo run`。
5. 用 Beancount 工具链做语法与平账检查（可选但推荐）。

### 2.2 常用命令

```bash
cargo run -- --help

# 用白盒样本做快速回归
cargo run -- --provider alipay --source testsets/支付宝交易明细测试数据集.csv --config config/third_party/alipay.yml --output tmp/output/dev-alipay.beancount --log-level info
cargo run -- --provider yinhe --source <your-yinhe-statement.xlsx> --config config/securities/yinhe.yml --output tmp/output/dev-yinhe.beancount --log-level info

# 严格模式回归
cargo run -- --provider alipay --source testsets/支付宝交易明细测试数据集.csv --config config/third_party/alipay.yml --strict --log-level info
```

### 2.3 日志级别

- 日常：`--log-level info`；定位问题：`debug`；深入追踪：`trace`；只看错误：`error`。
- 关注日志行：`CSV/XLSX headers`、`parsing complete`、`Transformation complete: success / ignored / failed`。

### 2.4 典型问题排查

- **表头不匹配**（字段大量为空）：看日志 headers 是否与 mapping 列名一致，检查 `skip_header_lines`/`has_header_row`。
- **编码乱码**：配 `tabular_options.encoding` 为 `auto`/`UTF-8`/`GBK`，看日志"检测到编码"。
- **strict 差异**：非 strict 坏行告警跳过；strict 直接失败，适合 CI 把关。
- **卖出证券分录看不懂**：多出的 `pnl_account` 行是"已实现盈亏承接行"，不是额外手续费。

### 2.5 测试重点

- 单元测试：规则条件匹配、metadata key 归一化、金额解析与 transform、Writer 输出语法。
- 集成测试：不同 provider 真实样本、含说明头/多编码/列数不齐边界输入、strict 与非 strict 两种模式。

## 3. 发布流程

使用 **GitHub Releases + tag 触发的 GitHub Actions 自动构建**（`.github/workflows/release.yml`）。

### 3.1 发布前检查

```bash
cargo fmt --check
cargo test --quiet
```

至少跑两个端到端样本（1 个现金流 + 1 个证券），并确认 `CHANGELOG.md` 与 `Cargo.toml` 版本号已更新。

### 3.2 发布步骤

1. 提交本次版本相关变更（代码、配置、文档、changelog），版本号 `vMAJOR.MINOR.PATCH`。
2. 打 tag 并推送：

```bash
git tag v0.2.0
git push origin v0.2.0
```

3. 等待 Actions `Release` workflow 完成，到 Releases 页面检查平台包齐全、`SHA256SUMS` 存在、说明准确。
4. 从 Release 下载任意平台包做冒烟测试：`./beancount-importer-rust --help` + 一条导入命令。

### 3.3 回滚与热修

严重问题：立刻创建修复 commit → 重发补丁版本（如 `v0.2.1`），**不要覆盖已有 tag**。

## 4. 新增供应商

### 4.1 先判断是否需要写 Rust 代码

- 只是"同一 provider 的新账单模板/新列名变体" → 只改 YAML（`config/` + `mapping/`）。
- 全新 provider 名称 → 必须新增 Rust provider 并注册。

分类固定三种：`third_party` / `banks` / `securities`。

### 4.2 需要改哪些文件

新增现金流 provider：

1. `src/providers/<category>/<name>.rs`
2. `src/providers/<category>/mod.rs`（`pub mod <name>;` + `all()` 添加实例）
3. `src/runtime/provider_registry.rs`（`registry.register(...)`）
4. `config/<category>/<name>.yml`
5. `mapping/<category>/<name>.yml`

证券 provider 额外：`securities_accounts.cash_account`（强烈建议显式配置）+ `output.booking_method: "FIFO"`。

### 4.3 建议执行顺序

1. 确定 `provider id`（小写、稳定、不要改名）。
2. 复制最接近的现有 provider 文件为新文件。
3. 先接通 Rust 最小骨架（`name/description/transform`）。
4. 修改 `mod.rs` + `provider_registry` 注册。
5. 写 `mapping` 最小版（先保证 `date`、`amount` 可读）。
6. 写 `config` 最小版（`default.*` + 读取参数）。
7. 真实样本跑普通模式，确认产出 `.beancount`。
8. 同一样本跑 strict 模式，修掉解析/映射错误。
9. 再补 `rules` 和 `extra_fields`。
10. 最后 `cargo test --quiet`。

### 4.4 mapping 必填字段

- 现金流硬必填：`date`、`amount`；强烈建议 `transaction_type`（否则靠金额符号判方向，退款/冲正容易误判）。
- 证券硬必填：`date` +（买卖）`symbol`/`quantity`/`amount 或 unit_price`；银证转账 `amount`/`transaction_type`。

现金流 mapping 模板：

```yaml
date: "交易时间"
amount:
  column: "金额"
  transform: abs
transaction_type: "收/支"
payee: "交易对方"
narration: "摘要"
reference: "交易单号"
currency: "币种"
date_formats:
  - "%Y-%m-%d %H:%M:%S"
  - "%Y-%m-%d"
extra_fields:
  orderId: "交易单号"
  payTime: "交易时间"
```

证券 mapping 模板：

```yaml
date: "成交日期"
transaction_type: "业务类型"
symbol: "证券代码"
security_name: "证券名称"
quantity: "成交数量"
unit_price: "成交价格"
amount: "发生金额"
currency: "币种"
reference: "订单号"
fee: "手续费"
tax: "税费"
date_formats:
  - "%Y%m%d"
  - "%Y-%m-%d"
extra_fields:
  market: "市场"
  account: "账户"
```

### 4.5 provider 配置模板

现金流：

```yaml
name: "示例平台"
mapping_file: "mapping/<category>/<provider>.yml"

default:
  asset_account: "Assets:Wallet:<Provider>:Balance"
  expense_account: "Expenses:Unknown"
  income_account: "Income:Unknown"
  currency: "CNY"

tabular_options:
  delimiter: ","
  flexible: true
  encoding: "auto"

skip_header_lines: 0
has_header_row: true

output:
  date_format: "%Y-%m-%d"
  decimal_places: 2

rules: []
```

证券：

```yaml
name: "示例券商"
mapping_file: "mapping/securities/<provider>.yml"

default:
  asset_account: "Assets:Broker:<Provider>:Securities"
  expense_account: "Expenses:Investing:Fees"
  income_account: "Income:Investing:Capital-Gains"
  currency: "CNY"

securities_accounts:
  cash_account: "Assets:Broker:<Provider>:Cash"

tabular_options:
  delimiter: ","
  flexible: true
  encoding: "auto"

skip_header_lines: 0
has_header_row: true

output:
  date_format: "%Y-%m-%d"
  decimal_places: 4
  booking_method: "FIFO"

rules: []
```

### 4.6 Rust provider 模板

现金流：

```rust
use std::path::Path;

use crate::{
    error::ImporterResult,
    interface::provider::{Provider, parse_tabular_source},
    model::{
        config::provider::ProviderConfig,
        data::raw_record::RawRecord,
        mapping::field_mapping::FieldMapping,
        rule::rule_engine::RuleEngine,
        transaction::Transaction,
    },
    providers::shared::{CashflowTransformOptions, transform_cashflow_record},
};

const XXX_OPTIONS: CashflowTransformOptions = CashflowTransformOptions {
    default_asset_fallback: "Assets:XXX",
};

pub struct XxxProvider;

impl Provider for XxxProvider {
    fn name(&self) -> &'static str { "xxx" }

    fn description(&self) -> &'static str { "XXX statement importer" }

    fn display_name(&self) -> &'static str { "某某平台" }

    fn parse(
        &self,
        path: &Path,
        mapping: &FieldMapping,
        config: &ProviderConfig,
        strict_mode: bool,
    ) -> ImporterResult<Vec<RawRecord>> {
        parse_tabular_source(path, mapping, config, strict_mode)
    }

    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        transform_cashflow_record(
            self.name(),
            self.display_name(),
            XXX_OPTIONS,
            record,
            rule_engine,
            config,
        )
    }
}
```

证券：

```rust
use crate::{
    error::ImporterResult,
    interface::provider::Provider,
    model::{
        config::provider::ProviderConfig,
        data::raw_record::RawRecord,
        rule::rule_engine::RuleEngine,
        transaction::Transaction,
    },
    providers::shared::{SecurityTransformOptions, transform_security_record},
};

const XXX_OPTIONS: SecurityTransformOptions = SecurityTransformOptions {
    default_payee: "XXX",
};

pub struct XxxProvider;

impl Provider for XxxProvider {
    fn name(&self) -> &'static str { "xxx" }

    fn description(&self) -> &'static str { "XXX securities statement importer" }

    fn display_name(&self) -> &'static str { "某某证券" }

    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        transform_security_record(
            self.name(),
            self.display_name(),
            XXX_OPTIONS,
            record,
            rule_engine,
            config,
        )
    }
}
```

### 4.7 何时需要特化逻辑

90% 的接入只需共享转换器 + YAML。仅以下场景才在 provider 里做特化：上游交易类型必须先归一化、某类记录要走专门分录路径（如"利息归本无证券代码"）、必须注入平台专属字段修正。特化时先处理特例分支，剩余记录回落共享转换器，不要复制 shared 的完整分录逻辑。

### 4.8 新增标准字段（模型扩展）

先判断是否真需要：能否用 `extra_fields` 承载？能否用现有标准字段 + 规则表达？只有"必须强类型参与转换逻辑"（金额计算、证券分录构建）才升级标准字段。改动涉及 `raw_record.rs`、`field_mapping.rs`、`reader/tabular/*`、`rule_action.rs`/`match_result.rs` 等，评审成本高，需在 PR 里明确动机。

### 4.9 端到端验证

```bash
cargo run -- --provider <name> --source <sample> --config config/<category>/<name>.yml --output tmp/output/<name>.beancount --log-level info
cargo run -- --provider <name> --source <sample> --config config/<category>/<name>.yml --strict --output tmp/output/<name>-strict.beancount --log-level info
cargo test --quiet
```

现金流至少覆盖：支出、收入、退款/冲正、币种缺失、对手方缺失、应忽略记录。证券至少覆盖：买入、卖出、银证转入、银证转出、手续费/税费非零、缺 amount 但有 unit_price、应忽略记录。

### 4.10 常见失败与排查

| 报错/现象 | 常见根因 | 修复动作 |
|---|---|---|
| `Unknown provider` | 未注册或 `name()` 不一致 | 检查 `provider_registry.rs` 注册项和 `name()` |
| `No field mapping file found` | `mapping_file` 错或目录错 | 改为 `mapping/<category>/<provider>.yml` 并确认存在 |
| `Missing date` | `date` 列名/格式不匹配 | 修 `date` 映射并补 `date_formats` |
| `Missing amount` | 金额列未映射或无法解析 | 修 `amount` 映射、必要时 `transform: abs` |
| `Missing security symbol` | 证券交易未映射 `symbol` | 修 `symbol` 映射或将非交易类型先 `ignore` |
| strict 下 `field count mismatch` | 分隔符/表头设置错 | 修 `delimiter`、`skip_header_lines`、`has_header_row` |
| strict 下 `Mapping error` | `regex_extract` 非法或列名拼错 | 修正正则/列名后重跑 strict |
