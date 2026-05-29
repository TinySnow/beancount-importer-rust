# beancount-importer-rust

一个面向日常对账的 Rust CLI 工具，用于把银行、第三方支付、券商账单转换为可直接导入 Beancount 的复式分录。

## 1. 当前支持的供应商

- 第三方支付：`alipay`、`wechat`、`jd`、`mt`
- 银行：`icbc`、`ccb`、`dzccb`
- 证券：`futu`、`yinhe`

## 2. 核心能力

- 统一读取 `CSV` / `XLSX`，支持编码自动识别（如 UTF-8、GBK）。
- `provider + mapping + rules` 的配置化导入，不把表头和业务规则硬编码到 Rust 代码。
- 规则引擎支持 `equals/contains/regex/in/not_empty/is_empty/数值比较`。
- 规则执行顺序稳定：`priority -> specificity -> 文件顺序`，后命中覆盖先命中。
- 支持 `terminal`（命中后停止后续规则）和 `ignore`（忽略该条交易）。
- 证券场景支持：普通买卖、逆回购、银证转账；通过 `securities_accounts` 子结构统一配置 `cash/fee/pnl/rounding/repo_interest`。
- Writer 支持自动输出 `commodity`，可选自动输出 `open` 指令。
- metadata key 自动归一化为 Beancount 合法键。

## 3. 快速开始

### 3.1 下载 Release（二进制，推荐）

发布页：
- [GitHub Releases](https://github.com/TinySnow/beancount-importer-rust/releases)

下载与你平台匹配的压缩包后，解压并进入目录，直接运行：

```bash
./beancount-importer-rust --help
```

说明：
- 发布包会附带 `config/` 与 `mapping/`，可直接按示例命令运行。
- Windows 下请使用 `beancount-importer-rust.exe`。

### 3.2 从源码编译

```bash
cargo build --release
```

### 3.3 运行（支付宝示例）

```bash
cargo run -- \
  --provider alipay \
  --source testsets/支付宝交易明细测试数据集.csv \
  --config config/third_party/alipay.yml \
  --output tmp/output/out-alipay.beancount \
  --log-level info
```

### 3.4 运行（银河证券示例）

```bash
cargo run -- \
  --provider yinhe \
  --source <your-yinhe-statement.xls> \
  --config config/securities/yinhe.yml \
  --output tmp/output/out-yinhe.beancount \
  --log-level info
```

### 3.5 运行（达州银行示例）

```bash
cargo run -- \
  --provider dzccb \
  --source <your-dzccb-statement.xls> \
  --config config/banks/dzccb.yml \
  --output tmp/output/out-dzccb.beancount \
  --log-level info
```

### 3.6 证券账户最小配置（推荐）

```yaml
default:
  asset_account: "Assets:Broker:Galaxy:Securities"
  expense_account: "Expenses:Investing:Fees"
  income_account: "Income:Investing:Capital-Gains"
  currency: "CNY"

securities_accounts:
  cash_account: "Assets:Broker:Galaxy:Cash"
  # 可选：仅在需要细分时再加
  # fee_account: "Expenses:Broker:Galaxy:Fee"
  # pnl_account: "Income:Broker:Galaxy:PnL"
  # rounding_account: "Expenses:Broker:Galaxy:Rounding"
  # repo_interest_account: "Income:Broker:Galaxy:RepoInterest"
# inventory_seed_files:
#   - "C:/Users/<you>/Desktop/Beancount/transactions/2025/12/galaxy.bean"

output:
  emit_open_directives: true
  booking_method: "FIFO"  # 建议：跨账期导入时可避免 `{}` lot 二义性
```

说明：
- 通用默认项推荐写在 `default:` 分组下（如 `asset_account/expense_account/income_account/currency`）。
- 证券账户字段统一写在 `securities_accounts` 下（`cash/fee/pnl/rounding/repo_interest`）。
- 旧平铺证券账户字段（如 `default_cash_account/default_fee_account` 等）不再生效。
- 历史平铺字段 `default_asset_account/default_expense_account/default_income_account/default_currency` 已移除；继续使用会在加载配置时报错。
- `inventory_seed_files` 可选；用于跨账期导入时预加载历史 lot，减少早期卖出（本期无买入）的二义性。

## 4. CLI 参数

- `-p, --provider <PROVIDER>`：供应商标识。
- `-s, --source <SOURCE>`：账单文件路径（CSV/XLSX）。
- `-c, --config <CONFIG>`：provider 配置路径。
- `-g, --global-config <GLOBAL_CONFIG>`：全局配置路径。
- `-m, --mapping <MAPPING>`：覆盖字段映射文件（可选；未设置时自动使用内嵌 mapping）。
- `-o, --output <OUTPUT>`：输出文件路径（不填则输出到 stdout）。
- `--log-level <LEVEL>`：`error/warn/info/debug/trace`。
- `-q, --quiet`：等价 `--log-level error`。
- `-v, --verbose`：等价 `--log-level debug`。
- `--strict`：严格模式；任意一条记录解析或转换失败即立即退出。

## 5. 配置加载顺序

运行时按以下顺序加载：
1. 全局配置 `--global-config`（未显式指定时尝试 `config/global.yml`）。
2. provider 配置 `--config`（不存在时优先尝试 `config/<category>/<provider>.yml`）。
3. 字段映射：优先检查 `--mapping`，其次检查 provider 配置中的 `mapping_file`，再次尝试分层路径 `mapping/<category>/<provider>.yml`。
4. 若以上均未命中，回退到编译期内嵌 mapping（仅内置供应商）。
5. 若仍找不到 mapping 文件，视为错误退出。

补充：provider 默认值会覆盖 global；provider 未设置的字段回退到 global。

## 6. 目录结构（与当前代码一致）

```text
src/
  main.rs
  lib.rs
  runtime/
    cli/
  interface/
  model/
  providers/
    banks/
    third_party/
    securities/
    shared/
config/
  global.yml
  banks/*.yml
  third_party/*.yml
  securities/*.yml
mapping/
  banks/*.yml
  third_party/*.yml
  securities/*.yml
examples/
  banks/<provider>/{basic.yml,advanced.yml}
  third_party/<provider>/{basic.yml,advanced.yml}
testsets/
  *.csv
  白盒测试数据集说明.md
docs/
  架构设计.md
  配置详解.md
  供应商扩展指南.md
  开发与调试手册.md
scripts/
  autopush.sh
```

## 7. 已验证的数据集

已用分层配置（`config/<category>/*.yml + mapping/<category>/*.yml`）跑通以下 6 份白盒数据集：
- `testsets/支付宝交易明细测试数据集.csv`（23）
- `testsets/微信支付账单测试数据集.csv`（23）
- `testsets/京东交易流水测试数据集.csv`（22）
- `testsets/美团账单测试数据集.csv`（22）
- `testsets/工商银行交易明细测试数据集.csv`（23）
- `testsets/建设银行交易明细测试数据集.csv`（23）

## 8. 文档索引

- [发布流程](docs/发布流程.md)
- [架构设计](docs/架构设计.md)
- [配置详解](docs/配置详解.md)
- [配置最佳实践指南](docs/配置最佳实践指南.md)
- [供应商扩展指南](docs/供应商扩展指南.md)
- [开发与调试手册](docs/开发与调试手册.md)
- [示例配置说明](examples/README.md)
- [白盒测试数据集说明](testsets/白盒测试数据集说明.md)

## 9. 质量检查

```bash
cargo fmt --check
cargo test --quiet
```

## 10. License

MIT
