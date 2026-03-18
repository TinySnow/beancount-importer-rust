//! Beancount 写入器：负责把标准交易渲染为 Beancount 文本。

mod directives;
mod render;

#[cfg(test)]
mod tests;

use std::{collections::BTreeSet, io::Write};

use crate::model::{config::output::OutputConfig, transaction::Transaction};

#[derive(Debug, Default)]
struct OpenAccountInfo {
    fiat_currencies: BTreeSet<String>,
    has_non_fiat: bool,
}

/// Beancount 文本写出器。
pub struct BeancountWriter {
    config: OutputConfig,
}

impl BeancountWriter {
    /// 创建写出器。
    pub fn new(config: OutputConfig) -> Self {
        Self { config }
    }

    /// 按配置把交易集合写出为 Beancount 文本。
    ///
    /// 输出顺序：
    /// 1. （可选）`open` 指令；
    /// 2. `commodity` 指令；
    /// 3. 逐笔交易。
    pub fn write(
        &self,
        transactions: &[Transaction],
        writer: &mut dyn Write,
    ) -> std::io::Result<()> {
        if self.config.emit_open_directives {
            self.write_open_directives(transactions, writer)?;
        }

        self.write_commodity_directives(transactions, writer)?;

        for (index, tx) in transactions.iter().enumerate() {
            if index > 0 {
                writeln!(writer)?;
            }
            self.write_transaction(tx, writer)?;
        }

        Ok(())
    }
}
