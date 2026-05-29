use std::{fs, path::Path};

use serde::de::DeserializeOwned;

use crate::error::{ImporterError, ImporterResult};

/// 读取并解析一个 YAML 文件。
///
/// 额外处理 UTF-8 BOM，避免某些编辑器保存后导致解析失败。
pub(super) fn load_yaml_file<T>(path: &Path, label: &str) -> ImporterResult<T>
where
    T: DeserializeOwned,
{
    let content = fs::read_to_string(path).map_err(|e| {
        ImporterError::Io(e).with_context(format!("Failed to read {}: {}", label, path.display()))
    })?;

    // 某些 YAML 文件可能包含 BOM，解析前先剥离。
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);

    serde_yaml::from_str(content).map_err(|e| {
        ImporterError::Yaml(e).with_context(format!("Failed to parse {}: {}", label, path.display()))
    })
}
