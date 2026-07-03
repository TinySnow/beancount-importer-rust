# beancount-importer-rust

一个面向日常对账的 Rust CLI 工具，用于把银行、第三方支付、券商账单转换为可直接导入 Beancount 的复式分录。

## 1. 当前支持的供应商

- 第三方支付：`alipay`、`wechat`、`jd`、`mt`
- 银行：`icbc`、`ccb`、`dzccb`
- 证券：`futu`、`yinhe`

> 导入账单后通常配合 [beancount-budget-tool](https://github.com/TinySnow/beancount-budget-tool) 做月度预算分析。

## 2. 核心能力

- 统一读取 `CSV` / `XLSX`，支持编码自动识别（如 UTF-8、GBK）。
- `provider + mapping + rules` 的配置化导入，不把表头和业务规则硬编码到 Rust 代码。
- 规则引擎支持 `equals/contains/regex/in/not_empty/is_empty/数值比较`。
- 规则执行顺序稳定：`priority -> specificity -> 文件顺序`，后命中覆盖先命中。
- 支持 `terminal`（命中后停止后续规则）和 `ignore`（忽略该条交易）。
- 证券场景支持：普通买卖、逆回购、银证转账；通过 `securities_accounts` 子结构统一配置 `cash/fee/pnl/rounding/repo_interest`。
- Writer 支持自动输出 `commodity`，可选自动输出 `open` 指令。
- metadata key 自动归一化为 Beancount 合法键。
- **批量导入模式**：`--batch batch.yml` 一次跑完当月全部 provider，取代逐条手写命令。

## 3. 快速开始（5 分钟上手）

### 3.1 下载二进制（推荐）

从 [GitHub Releases](https://github.com/TinySnow/beancount-importer-rust/releases) 下载对应平台的压缩包，解压后直接运行：

```bash
./beancount-importer-rust --help
```

发布包自带 `config/` 和 `mapping/` 目录，可直接使用。Windows 下二进制名为 `beancount-importer-rust.exe`。

### 3.2 从源码编译

```bash
cargo build --release
```

### 3.3 运行第一条导入（支付宝示例）

```bash
```bash
# 1. 从支付宝 App 导出账单 → 得到 alipay_2026.csv
# 2. 运行
./beancount-importer-rust \
  -p alipay \
  -s ~/Downloads/alipay_2026.csv \
  -c config/third_party/alipay.yml \
  -o alipay.bean \
  --log-level info
```

### 3.4 运行证券导入（银河证券示例）

```bash
./beancount-importer-rust \
  -p yinhe \
  -s ~/Downloads/yinhe_2026.xls \
  -c config/securities/yinhe.yml \
  -o galaxy.bean \
  --log-level info
```

### 3.5 账单导出位置

| 供应商 | 导出方式 |
|--------|---------|
| 支付宝 | App → 我的 → 账单 → 右上角 ··· → 开具交易流水证明 |
| 微信 | App → 我 → 服务 → 钱包 → 账单 → 右上角 ··· → 开具交易流水证明 |
| 京东 | App → 我的 → 我的钱包 → 账单 → 导出 |
| 美团 | App → 我的 → 钱包 → 账单 → 导出 |
| 工商银行 | 手机银行 → 我的账户 → 交易明细 → 导出 |
| 建设银行 | 手机银行 → 账户详情 → 交易明细 → 导出 |
| 达州银行 | 手机银行 → 交易明细 → 导出 |
| 银河证券 | 双子星/海王星 → 历史成交 → 导出为 XLS |

## 4. 批量导入（月度一键导入）

配置写好后，每月只用一条命令。更多示例见 `docs/配置最佳实践指南.md`。

```yaml
# batch-2026-06.yml
imports:
  - provider: icbc
    source: ~/Downloads/icbc-202606.csv
    config: config/banks/icbc.yml
    output: 2026/06/icbc.bean
  - provider: alipay
    source: ~/Downloads/alipay-202606.csv
    config: config/third_party/alipay.yml
    output: 2026/06/alipay.bean
```

```bash
./beancount-importer-rust --batch batch-2026-06.yml
```

## 5. CLI 参数

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

## 6. 配置加载顺序

运行时按以下顺序加载：
1. 全局配置 `--global-config`（未显式指定时尝试 `config/global.yml`）。
2. provider 配置 `--config`（不存在时优先尝试 `config/<category>/<provider>.yml`）。
3. 字段映射：优先检查 `--mapping`，其次检查 provider 配置中的 `mapping_file`，再次尝试分层路径 `mapping/<category>/<provider>.yml`。
4. 若以上均未命中，回退到编译期内嵌 mapping（仅内置供应商）。
5. 若仍找不到 mapping 文件，视为错误退出。

补充：provider 默认值会覆盖 global；provider 未设置的字段回退到 global。

## 7. 目录结构（与当前代码一致）

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

## 8. 已验证的数据集

已用分层配置（`config/<category>/*.yml + mapping/<category>/*.yml`）跑通以下 6 份白盒数据集：
- `testsets/支付宝交易明细测试数据集.csv`（23）
- `testsets/微信支付账单测试数据集.csv`（23）
- `testsets/京东交易流水测试数据集.csv`（22）
- `testsets/美团账单测试数据集.csv`（22）
- `testsets/工商银行交易明细测试数据集.csv`（23）
- `testsets/建设银行交易明细测试数据集.csv`（23）

## 9. 文档索引

- [发布流程](docs/发布流程.md)
- [架构设计](docs/架构设计.md)
- [配置详解](docs/配置详解.md)
- [配置最佳实践指南](docs/配置最佳实践指南.md)
- [供应商扩展指南](docs/供应商扩展指南.md)
- [开发与调试手册](docs/开发与调试手册.md)
- [示例配置说明](examples/README.md)
- [白盒测试数据集说明](testsets/白盒测试数据集说明.md)

## 10. 质量检查

```bash
cargo fmt --check
cargo test --quiet
```

## 11. License

MIT
