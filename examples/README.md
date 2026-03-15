# examples 配置示例

本目录按 provider 提供两套配置：
- `basic.yml`：基础版，适合快速上手。
- `advanced.yml`：高级版，演示复杂规则能力。

## 目录结构

- `examples/alipay/{basic.yml,advanced.yml}`
- `examples/wechat/{basic.yml,advanced.yml}`
- `examples/jd/{basic.yml,advanced.yml}`
- `examples/mt/{basic.yml,advanced.yml}`
- `examples/icbc/{basic.yml,advanced.yml}`
- `examples/ccb/{basic.yml,advanced.yml}`

## 快速说明（简洁版）

- `basic.yml`：状态过滤 + 高频分类 + 兜底规则。
- `advanced.yml`：演示 `regex/in/数值比较/not_empty/is_empty`、`match_mode`、`priority/terminal/ignore` 等。
- 单条规则只支持 `and` 或 `or`，不支持括号嵌套；复杂场景请用 `in/regex` 或拆多条规则。

## 详细语义

完整语义说明见：[docs/配置详解.md](../docs/配置详解.md)

## 验证命令（基础版）

```bash
cargo run -- --provider alipay --source testsets/支付宝交易明细测试数据集.csv --config examples/alipay/basic.yml --output tmp/output/examples-basic-alipay.beancount --log-level info
cargo run -- --provider wechat --source testsets/微信支付账单测试数据集.csv --config examples/wechat/basic.yml --output tmp/output/examples-basic-wechat.beancount --log-level info
cargo run -- --provider jd --source testsets/京东交易流水测试数据集.csv --config examples/jd/basic.yml --output tmp/output/examples-basic-jd.beancount --log-level info
cargo run -- --provider mt --source testsets/美团账单测试数据集.csv --config examples/mt/basic.yml --output tmp/output/examples-basic-mt.beancount --log-level info
cargo run -- --provider icbc --source testsets/工商银行交易明细测试数据集.csv --config examples/icbc/basic.yml --output tmp/output/examples-basic-icbc.beancount --log-level info
cargo run -- --provider ccb --source testsets/建设银行交易明细测试数据集.csv --config examples/ccb/basic.yml --output tmp/output/examples-basic-ccb.beancount --log-level info
```

## 验证命令（高级版）

```bash
cargo run -- --provider alipay --source testsets/支付宝交易明细测试数据集.csv --config examples/alipay/advanced.yml --output tmp/output/examples-advanced-alipay.beancount --log-level info
cargo run -- --provider wechat --source testsets/微信支付账单测试数据集.csv --config examples/wechat/advanced.yml --output tmp/output/examples-advanced-wechat.beancount --log-level info
cargo run -- --provider jd --source testsets/京东交易流水测试数据集.csv --config examples/jd/advanced.yml --output tmp/output/examples-advanced-jd.beancount --log-level info
cargo run -- --provider mt --source testsets/美团账单测试数据集.csv --config examples/mt/advanced.yml --output tmp/output/examples-advanced-mt.beancount --log-level info
cargo run -- --provider icbc --source testsets/工商银行交易明细测试数据集.csv --config examples/icbc/advanced.yml --output tmp/output/examples-advanced-icbc.beancount --log-level info
cargo run -- --provider ccb --source testsets/建设银行交易明细测试数据集.csv --config examples/ccb/advanced.yml --output tmp/output/examples-advanced-ccb.beancount --log-level info
```