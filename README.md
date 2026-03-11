# beancount-importer-rust

一个面向生产使用场景的 Rust CLI 工具，用于把银行、第三方支付、券商流水转换为可直接导入 Beancount 的复式记账分录。

## 1. 项目定位

本项目关注三个目标：

- 可维护：通过 `provider + mapping + rules` 组合降低硬编码
- 可迁移：metadata 命名尽量与 double-entry-generator 对齐
- 可扩展：新增供应商时只需要实现 Provider 并提供配置

## 2. 关键能力

- 多供应商导入：支付宝、微信、京东、美团、工商银行、建设银行、富途、银河证券
- 规则引擎：支持 `equals/contains/regex/in/notEmpty/isEmpty/数值比较`
- 规则优先级：按 `priority -> specificity -> 文件顺序` 应用，后匹配覆盖先匹配
- 支持 `terminal`：命中后可提前终止（默认 `false`）
- 证券交易支持：
  - 基金/证券通货符号自动规范化（如 `FUND_159915` / `SEC_204001`）
  - 卖出使用 `{}` 推断成本并支持 PnL 账户
  - 回购交易（逆回购）专门建模
  - 支持手续费、尾差、PnL 的 provider 默认账户 + 规则级覆盖
- 输出增强：
  - 自动输出 `open` 指令（可选）
  - 自动输出 `commodity` 指令
  - metadata key 自动归一化为 Beancount 合法键

## 3. 元数据命名约定（迁移友好）

为了减少迁移成本，项目默认将常见中文字段映射为如下 metadata key：

- `交易状态 -> status`
- `来源 -> source`
- `交易分类 -> txType`（支付宝映射为 `category`）
- `收/支 -> type`
- `交易时间 -> payTime`
- `收/付款方式 -> method`
- `交易订单号 -> orderId`
- `商家订单号 -> merchantId`
- `交易对方 -> peer`
- `对方账号 -> peerAccount`

兼容性说明：

- 旧 key（如 `counterparty/counterpartyAccount`）仍可被兼容识别
- 输出会统一落到新 key（`peer/peerAccount`）

## 4. 项目结构

```text
beancount-importer-rust/
├─ src/
│  ├─ main.rs                      # CLI 入口
│  ├─ lib.rs                       # 库入口
│  ├─ runtime/                     # 运行时编排（配置加载 + 导入流程）
│  ├─ interface/                   # Provider trait 定义
│  ├─ providers/                   # 供应商实现（必须放这里）
│  │  ├─ banks/                    # 银行类
│  │  ├─ third_party/              # 第三方支付类
│  │  └─ securities/               # 证券类
│  ├─ model/                       # 领域模型（交易、规则、映射、输出）
│  ├─ utils/                       # 工具模块（编码、金额、日期、metadata）
│  ├─ config/                      # provider 配置示例
│  └─ mapping/                     # 字段映射示例
├─ tests/                          # 脱敏测试样本
└─ docs/                           # 详细中文文档
```

## 5. 快速开始

### 5.1 编译

```bash
cargo build --release
```

### 5.2 运行（示例：支付宝）

```bash
cargo run -- \
  --provider alipay \
  --source tests/支付宝交易明细测试数据集.csv \
  --config src/config/alipay.yml \
  --output out.beancount
```

### 5.3 运行（示例：银河证券）

```bash
cargo run -- \
  --provider yinhe \
  --source tests/银河证券-20260101_20260308.xls \
  --config src/config/yinhe.yml \
  --output out-yinhe.beancount
```

## 6. CLI 参数

- `-p, --provider <PROVIDER>`：供应商名
- `-s, --source <SOURCE>`：流水文件路径
- `-c, --config <CONFIG>`：provider 配置路径（默认 `config.yml`）
- `-g, --global-config <GLOBAL_CONFIG>`：全局配置路径
- `-o, --output <OUTPUT>`：输出路径，不填则输出到 stdout
- `--log-level <LEVEL>`：日志级别（trace/debug/info/warn/error）
- `-q, --quiet`：等价 `--log-level error`
- `-v, --verbose`：等价 `--log-level debug`

## 7. 配置加载顺序

导入时配置按如下顺序解析：

1. 全局配置 `--global-config`
2. provider 配置 `--config`
3. provider 内声明的 mapping 文件
4. fallback 的 `src/mapping/<provider>.yml`

字段缺省规则：

- provider 配置优先于 global 配置
- global 不存在则使用内置默认值

## 8. 证券账户扩展配置

在 provider 配置中可定义以下账户：

- `default_fee_account`
- `default_pnl_account`
- `default_rounding_account`

规则级可覆盖：

- `rules[].action.fee_account`
- `rules[].action.pnl_account`
- `rules[].action.rounding_account`

覆盖优先级：

1. 规则级覆盖
2. provider 默认扩展账户
3. provider 通用默认账户（expense/income）
4. 内置 fallback

## 9. 供应商 source 标签

每条导出的交易都会注入 `source` metadata。

例如：

- wechat -> `source: "微信"`
- alipay -> `source: "支付宝"`
- yinhe -> `source: "银河证券"`

## 10. 质量保障

```bash
cargo fmt
cargo test --quiet
```

建议每次改动后至少执行一次上述命令。

## 11. 常见问题

### Q1: Beancount 报 metadata key 非法

A: 本项目在写出前会做 key 规范化；如果你在规则里写了特殊字符 key，请改成字母开头的 ASCII 标识符。

### Q2: 证券卖出分录报不平

A: 检查是否缺少 PnL 行或手续费行；项目默认会按 `fee/pnl/rounding` 账户补齐。

### Q3: 通货符号报错“必须以字母开头”

A: 项目已自动处理纯数字代码；基金会生成 `FUND_XXXXXX`，其他证券生成 `SEC_XXXXXX`。

## 12. 详细文档索引

- [架构设计](docs/架构设计.md)
- [配置详解](docs/配置详解.md)
- [供应商扩展指南](docs/供应商扩展指南.md)
- [开发与调试手册](docs/开发与调试手册.md)

## 13. License

MIT