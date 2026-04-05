use std::path::{Path, PathBuf};

use crate::model::config::provider::ProviderConfig;

/// 判断字符串路径是否应视为“已绝对化路径”（跨平台）。
///
/// 除了 `Path::is_absolute()`，还兼容：
/// - Windows 盘符路径：`C:/...`、`C:\...`
/// - Windows UNC 路径：`\\server\share\...`
fn is_effectively_absolute_path(raw: &str) -> bool {
    let candidate = Path::new(raw);
    if candidate.is_absolute() {
        return true;
    }

    let bytes = raw.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\')
    {
        return true;
    }

    raw.starts_with("\\\\")
}

/// 将 `inventory_seed_files` 的相对路径解析为相对配置文件目录的绝对路径。
///
/// 绝对路径（含 Windows 盘符/UNC）保持不变；相对路径会被重写为
/// `config_base_path` 所在目录下的路径字符串。
pub(super) fn resolve_inventory_seed_paths(
    provider_config: &mut ProviderConfig,
    config_base_path: &Path,
) {
    if provider_config.inventory_seed_files.is_empty() {
        return;
    }

    let base_dir = config_base_path.parent().unwrap_or_else(|| Path::new("."));
    provider_config.inventory_seed_files = provider_config
        .inventory_seed_files
        .iter()
        .map(|raw| {
            let candidate = PathBuf::from(raw);
            if is_effectively_absolute_path(raw) {
                candidate
            } else {
                base_dir.join(candidate)
            }
        })
        .map(|path| path.to_string_lossy().to_string())
        .collect();
}
