# beancount-importer-rust

把银行账单、第三方支付、券商交易记录转成 [Beancount](https://beancount.github.io/) 复式分录的 CLI 工具。

## 支持的供应商

```
第三方支付    alipay   wechat   jd   mt
银行          icbc     ccb      dzccb
证券          yinhe    futu
```

## 安装

从 [Releases](https://github.com/TinySnow/beancount-importer-rust/releases) 下载对应平台的二进制，解压即用。

```bash
./beancount-importer-rust --help
```

从源码编译：

```bash
cargo build --release
```

## 5 分钟上手

**Step 1** — 导出账单（支付宝 → 支付宝 → 账单 → ··· → 开具交易流水证明）

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

**Step 3** — 配置规则自动分类。在 `config/third_party/alipay.yml` 加：

```yaml
rules:
  - name: "餐饮"
    conditions:
      - fields: [peer, item]
        regex: "星巴克|麦当劳|外卖"
    action:
      debit_account: "Expenses:Food"
  - name: "交通"
    conditions:
      - field: "peer"
        contains: "滴滴|铁路|地铁"
    action:
      debit_account: "Expenses:Transport"
```

> 功能清单与完整示例见 **[QUICKSTART.md](QUICKSTART.md)**

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
```

```bash
./beancount-importer-rust --batch batch-2026-06.yml
```

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

## 文档

- **[QUICKSTART.md](QUICKSTART.md)** — 功能清单、规则写法、场景示例
- [配置详解](docs/配置详解.md) — 所有 YAML 字段说明
- [架构设计](docs/架构设计.md)
- [供应商扩展指南](docs/供应商扩展指南.md) — 添加新数据源

## License

MIT
