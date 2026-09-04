# beancount-importer-rust

把银行账单、第三方支付、券商交易记录转成 [Beancount](https://beancount.github.io/) 复式分录的 CLI 工具。

## 支持的供应商

```
第三方支付    alipay   wechat   jd   mt
银行          icbc     ccb      dzccb
证券          yinhe    futu
```

## 安装

从 [Releases](https://github.com/TinySnow/beancount-importer-rust/releases) 下载对应平台的二进制，解压即用（内含 `config/` 与 `mapping/` 配置模板）。

```bash
./beancount-importer-rust --help
```

从源码编译：

```bash
cargo build --release
```

## 5 分钟上手

**Step 1** — 导出账单（支付宝 → 我的 → 账单 → 开具交易流水证明）。

**Step 2** — 一条命令：

```bash
./beancount-importer-rust \
  -p alipay \
  -s ~/Downloads/alipay_2026.csv \
  -c config/third_party/alipay.yml \
  -o alipay.bean --log-level info
```

输出类似：

```beancount
2026-06-15 * "星巴克" "Coffee"
  source: "支付宝"
  orderId: "20260615..."
  Expenses:Food:Coffee  32.50 CNY
  Assets:Wallet:Alipay:Balance  -32.50 CNY
```

**Step 3** — 用规则自动分类。在 `config/third_party/alipay.yml` 加：

```yaml
rules:
  - name: "餐饮"
    conditions:
      - fields: [peer, item]
        regex: "星巴克|麦当劳|外卖"
    action:
      debit_account: "Expenses:Food"
```

## 规则引擎

### 基本写法

```yaml
rules:
  - name: "餐饮"
    conditions:
      - field: "peer"
        equals: "星巴克"        # 精确匹配
    action:
      debit_account: "Expenses:Food"

  - name: "外卖"
    conditions:
      - field: "peer"
        contains: "美团"         # 子串匹配
    action:
      debit_account: "Expenses:Food:Delivery"

  - name: "交通"
    conditions:
      - field: "peer"
        regex: "滴滴|铁路|地铁|航空"    # 正则
    action:
      debit_account: "Expenses:Transport"
```

### 多字段 OR 匹配（推荐 `fields` 数组）

```yaml
  - name: "餐饮"
    conditions:
      - fields: [peer, item]     # peer 或 item 任意命中即匹配
        regex: "小吃|零食|火锅|串串|餐饮|中餐|奶茶|咖啡|外卖"
    action:
      debit_account: "Expenses:Food"
```

一条条件覆盖两个字段，不用每个字段写一遍。

### 金额范围

```yaml
  - name: "小额杂项"
    conditions:
      - field: "amount"
        less_than: 50
    action:
      debit_account: "Expenses:Misc"
      ignore: true               # 忽略小额交易，不导出
```

### 全部运算符

| 运算符 | YAML | 说明 |
|---|---|---|
| 精确匹配 | `equals: "支出"` | 完全相等 |
| 子串 | `contains: "交通"` | 包含即命中 |
| 正则 | `regex: "咖啡\|奶茶"` | 支持捕获组 `{1}` `{2}` |
| 前缀 | `starts_with: "银行卡"` | |
| 后缀 | `ends_with: "-TEST"` | |
| 列表 | `in: ["A", "B", "C"]` | 值在列表中 |
| 非空 | `not_empty:` | 字段存在且非空 |
| 为空 | `is_empty:` | 字段不存在或为空 |
| 大于 | `greater_than: 100` | |
| 小于 | `less_than: 100` | |
| 区间 | `between: { min: 50, max: 200 }` | 闭区间 |

### 规则合并与优先级

多条规则命中同一笔交易时：

| 字段类型 | 合并方式 |
|---|---|
| `debit/credit_account`、`narration`、`payee`、`flag` | 后命中**覆盖**先命中 |
| `tags`、`links` | **追加**（不重复） |
| `metadata` | key 级合并，后 key 覆盖先 key |
| `ignore` | 一旦 true 不变 |
| `terminal` | 命中后**停止**所有后续规则 |

默认执行顺序：`priority 小 → 大` → `条件数 少 → 多` → `文件顺序`。

### 分层规则（推荐策略）

不要为"账号 × 品类"写 N×M 条规则。拆两层：

```yaml
rules:
  # Layer 1 — priority 100：识别支付方式 → 确定贷方账户
  - name: "支付宝"
    priority: 100
    conditions:
      - field: "source"
        equals: "支付宝"
    action:
      credit_account: "Assets:Wallet:Alipay:Balance"

  - name: "微信"
    priority: 100
    conditions:
      - field: "source"
        equals: "微信"
    action:
      credit_account: "Assets:Wallet:WeChat:Balance"

  # Layer 2 — priority 10：识别消费类别 → 确定借方账户
  - name: "餐饮"
    priority: 10
    conditions:
      - fields: [peer, item]
        regex: "小吃|零食|火锅|中餐|奶茶|外卖"
    action:
      debit_account: "Expenses:Food"
```

两层自动合并：支付宝买外卖 → debit=Expenses:Food, credit=Assets:Wallet:Alipay:Balance。N + M 条替代 N×M 条。

## 配置概览

每个供应商需要两个文件（Release 包已内置，修改即可）。

### 1. 字段映射（mapping）— 告诉工具 CSV 哪列是什么

```yaml
# mapping/third_party/alipay.yml
date: "交易时间"
payee: "交易对方"
narration: "商品说明"
transaction_type: "收/支"
amount:
  column: "金额"
  transform: abs           # abs: 取绝对值  negate: 取反
reference: "交易订单号"
date_formats:              # 按顺序尝试
  - "%Y/%m/%d %H:%M"
  - "%Y-%m-%d %H:%M:%S"
```

### 2. Provider 配置 — 告诉工具怎么读 + 怎么分类

```yaml
# config/third_party/alipay.yml
name: "支付宝"
default:
  asset_account: "Assets:Wallet:Alipay:Balance"
  expense_account: "Expenses:Unknown"
  income_account: "Income:Unknown"
  currency: "CNY"
tabular_options:
  delimiter: ","
  flexible: true
  encoding: "auto"
skip_header_lines: 24      # 跳过支付宝的 24 行说明
has_header_row: true
rules: []
```

银行类注意 `tabular_options` 可能需要 `income_column` / `expense_column`（出入账分两列），证券类需要 `securities_accounts`。完整字段见 **[CONFIGURATION.md](CONFIGURATION.md)**。

### 3. 全局配置 — 所有供应商的公共默认值（通常不用改）

```yaml
# config/global.yml
default:
  currency: CNY
  expense_account: Expenses:Unknown
  asset_account: Assets:Unknown
  income_account: Income:Unknown
output:
  date_format: "%Y-%m-%d"
  decimal_places: 2
```

## 每月一键导入（批量模式）

配置写好后每月只需要一条命令：

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
  - provider: yinhe
    source: ~/Downloads/yinhe-202606.xls
    config: config/securities/yinhe.yml
    output: 2026/06/galaxy.bean
```

```bash
./beancount-importer-rust --batch batch-2026-06.yml
```

所有相对路径基于 batch 文件所在目录解析。

## 证券专项

### 银河证券（`yinhe`）

自动识别：
- **银证转账**（`银行转证券`/`证券转银行`）→ broker ↔ bank 账户转移
- **逆回购**（`204001` / `131810` 或 `回购` 关键字）→ 固定面值 100 CNY/张，利息差额入 `repo_interest_account`
- **普通买卖** → 持仓带 cost/price，PnL 自动计算
- **份额分拆** → 自动移除旧份额 + 新增新份额，成本按旧份额成本推算
- **利息归本** → 专用处理链

```bash
./beancount-importer-rust -p yinhe \
  -s ~/Downloads/yinhe-202606.xls \
  -c config/securities/yinhe.yml \
  -o galaxy.bean --log-level info
```

输出示例：

```beancount
2026-06-15 * "银证转账" "银行转证券"
  source: "银河证券"
  Assets:Broker:Galaxy:Cash  10000.00 CNY
  Assets:Bank:ICBC:Savings  -10000.00 CNY

2026-06-16 * "买入" "沪深300ETF"
  source: "银河证券"
  symbol: "510300"
  Assets:Broker:Galaxy:Securities  1000 HOOD {4.567 CNY}
  Assets:Broker:Galaxy:Cash  -4567.00 CNY
  Expenses:Broker:Galaxy:Fee  5.00 CNY
  Expenses:Broker:Galaxy:Rounding  0.01 CNY
```

### 富途证券（`futu`）

USD 计价，自动输出 `commodity USD` 指令。

## 账单导出位置

| 供应商 | App 路径 |
|---|---|
| 支付宝 | 我的 → 账单 → ··· → 开具交易流水证明 |
| 微信 | 我 → 服务 → 钱包 → 账单 → ··· → 开具交易流水证明 |
| 京东 | 我的 → 我的钱包 → 账单 → 导出 |
| 美团 | 我的 → 钱包 → 账单 → 导出 |
| 工商银行 | 手机银行 → 我的账户 → 交易明细 → 导出 |
| 建设银行 | 手机银行 → 账户详情 → 交易明细 → 导出 |
| 达州银行 | 手机银行 → 交易明细 → 导出 |
| 银河证券 | 双子星/海王星 → 历史成交 → 导出为 XLS |

## CLI

| 参数 | 说明 |
|---|---|
| `-p, --provider` | 供应商 (`alipay\|wechat\|icbc\|yinhe\|...`) |
| `-s, --source` | 账单文件 (CSV/XLS/XLSX) |
| `-c, --config` | provider 配置文件路径 |
| `-g, --global-config` | 全局配置路径（默认 `config/global.yml`） |
| `-m, --mapping` | 字段映射文件（默认自动查找） |
| `-o, --output` | 输出路径（默认 stdout） |
| `--log-level` | `error\|warn\|info\|debug\|trace` |
| `--strict` | 任意记录失败即退出 |
| `-b, --batch` | 批量导入模式 |

## 常见调试

```bash
--log-level info     # 看到每条记录的分类结果
--log-level debug    # 看到规则匹配详情
--log-level trace    # 看到完整内部状态
--strict             # 任何一条记录解析失败就报错退出
```

```bash
# 只测试不写文件（不传 -o 输出到终端）
./beancount-importer-rust -p alipay -s bill.csv -c config/third_party/alipay.yml

# 验证输出
bean-check alipay.bean
grep "Expenses:Food" alipay.bean | wc -l
```

## 文档

- **[CONFIGURATION.md](CONFIGURATION.md)** — 所有可配置项、规则语义、账户回退、最佳实践
- **[CONTRIBUTING.md](CONTRIBUTING.md)** — 架构设计、开发调试、发布流程、新增供应商

## License

MIT
