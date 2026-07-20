# QUICKSTART — 功能清单与用法速查

> 只会用一两个供应商时，看前半部分（规则怎么写）。记不清怎么配新供应商时，看后半部分（配置速查）。

---

## 一、你需要准备两样东西

```
beancount-importer-rust \
  -p <provider> \          # ← ① 供应商名 (alipay, icbc, yinhe, ...)
  -s <账单文件> \            # ← ② 从 App 导出的 CSV / XLSX
  -c config/<category>/<provider>.yml \
  -o output.bean
```

配置文件（`config/` + `mapping/`）已内置在 Release 包中，直接用即可。

---

## 二、规则引擎 — 自动分类交易

### 基本写法

```yaml
# config/third_party/alipay.yml
rules:
  - name: "餐饮"
    conditions:
      - field: "peer"        # 单字段精确匹配
        equals: "星巴克"
    action:
      debit_account: "Expenses:Food"

  - name: "外卖"
    conditions:
      - field: "peer"
        contains: "美团"     # 子串匹配
    action:
      debit_account: "Expenses:Food:Delivery"

  - name: "交通"
    conditions:
      - field: "peer"
        regex: "滴滴|铁路|地铁|航空"    # 正则
    action:
      debit_account: "Expenses:Transport"
```

### 多字段 OR 匹配（推荐用 `fields` 数组）

```yaml
  - name: "餐饮"
    conditions:
      - fields: [peer, item]     # peer 或 item 任意命中即匹配
        regex: "小吃|零食|火锅|串串|餐饮|中餐|奶茶|咖啡|外卖"
    action:
      debit_account: "Expenses:Food"
```

一个条件覆盖 payee 和 narration 两个字段，不用每个字段写一遍。

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
  # Layer 1 — 优先级 100：识别支付方式 → 确定贷方账户
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

  # Layer 2 — 优先级 10：识别消费类别 → 确定借方账户
  - name: "餐饮"
    priority: 10
    conditions:
      - fields: [peer, item]
        regex: "小吃|零食|火锅|中餐|奶茶|外卖"
    action:
      debit_account: "Expenses:Food"
```

两层自动合并：支付宝买外卖 → debit=Expenses:Food, credit=Assets:Wallet:Alipay:Balance。

---

## 三、配置三段式

每个供应商需要两个文件。Release 包中已内置，修改即可。

### 1. 字段映射 — 告诉工具 CSV 哪列是什么

```yaml
# mapping/third_party/alipay.yml
date: "交易时间"
payee: "交易对方"
narration: "商品说明"
transaction_type: "收/支"
amount:
  column: "金额"
  transform: abs           # abs: 取绝对值  negate: 取反
status: "交易状态"
reference: "交易订单号"
date_formats:              # 按顺序尝试
  - "%Y/%m/%d %H:%M"
  - "%Y-%m-%d %H:%M:%S"
```

`amount` 支持更详细的写法：

```yaml
amount:
  column: "金额"
  transform: abs
  default: "0"             # 空值时回退
  regex_extract: "([0-9.]+)"  # 从文本中提取数字
```

### 2. Provider 配置 — 告诉工具怎么读 + 怎么分类

```yaml
# config/third_party/alipay.yml
name: "支付宝"
default:
  asset_account: "Assets:Wallet:Alipay:Balance"   # 默认资产账户
  expense_account: "Expenses:Unknown"             # 默认支出账户
  income_account: "Income:Unknown"                # 默认收入账户
  currency: "CNY"
tabular_options:           # CSV 解析参数
  delimiter: ","
  flexible: true           # 容忍列数不一致
  encoding: "auto"         # 自动检测 UTF-8 / GBK
skip_header_lines: 24      # 跳过支付宝的 24 行废话
has_header_row: true
rules: []                  # 规则列表
```

**银行类**注意：`tabular_options` 需要声明 `income_column` 和 `expense_column`（有些银行出入账分两列）：

```yaml
tabular_options:
  delimiter: ","
  flexible: true
  encoding: "auto"
  income_column: "收入金额"    # 工商银行等分列场景
  expense_column: "支出金额"
```

**证券类**额外需要 `securities_accounts`：

```yaml
securities_accounts:
  cash_account: "Assets:Broker:Galaxy:Cash"
  fee_account: "Expenses:Broker:Galaxy:Fee"
  pnl_account: "Income:Broker:Galaxy:PnL"
  rounding_account: "Expenses:Broker:Galaxy:Rounding"
  repo_interest_account: "Income:Investing:Interest"
output:
  decimal_places: 4          # 证券价格精度
  emit_open_directives: true # 自动输出 open 指令
  booking_method: "FIFO"
```

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

---

## 四、批量导入 — 每月一键

```yaml
# batch-2026-06.yml
imports:
  - provider: icbc
    source: ~/Downloads/icbc-202606.csv
    config: config/banks/icbc.yml
    output: 2026/06/icbc.bean

  - provider: ccb
    source: ~/Downloads/ccb-202606.csv
    config: config/banks/ccb.yml
    output: 2026/06/ccb.bean

  - provider: alipay
    source: ~/Downloads/alipay-202606.csv
    config: config/third_party/alipay.yml
    output: 2026/06/alipay.bean

  - provider: wechat
    source: ~/Downloads/wechat-202606.csv
    config: config/third_party/wechat.yml
    output: 2026/06/wechat.bean

  - provider: yinhe
    source: ~/Downloads/yinhe-202606.xls
    config: config/securities/yinhe.yml
    output: 2026/06/galaxy.bean
```

```bash
./beancount-importer-rust --batch batch-2026-06.yml
```

---

## 五、证券专项

### 银河证券（`yinhe`）— 懂中文券商

自动识别：
- **银证转账**（`银证转账` 关键字）→ broker ↔ bank 账户转移
- **逆回购**（`204001` / `131810` 或 `回购` 关键字）→ 固定面值 100 CNY/张，利息差额入 repo_interest_account
- **普通买卖** → holdings 带 cost 或 price，PnL 自动计算
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

### 富途证券（`futu`）— USD 计价

自动输出 `commodity USD` 指令。

---

## 六、常见调试场景

### 看导入过程

```bash
--log-level info     # 看到每条记录的分类结果
--log-level debug    # 看到规则匹配详情
--log-level trace    # 看到完整内部状态
```

### 只测试不写文件

```bash
# 不传 -o，输出到终端
./beancount-importer-rust -p alipay -s bill.csv -c config/third_party/alipay.yml
```

### 严格模式

```bash
--strict    # 任何一条记录解析失败就报错退出（不跳过错行）
```

### 验证输出

```bash
# 统计某账户出现次数
grep "Expenses:Food" alipay.bean | wc -l

# 用 beancount 语法检查
bean-check alipay.bean

# 配合 budget-tool 做预算分析
beancount-budget-tool -m 2026-06 --budgets budgets.yml --config config.yml --ledger 2026/06/*.bean
```
