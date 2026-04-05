//! `config_loader` 模块测试。
//!
//! 文件路径：`src/runtime/config_loader/tests.rs`。
//!
//! 本文件主要覆盖以下回归场景：
//! - 供应商名称大小写归一化与配置回退顺序；
//! - 映射文件从平铺目录迁移到分层目录时的兼容行为；
//! - `inventory_seed_files` 的跨平台绝对/相对路径处理；
//! - `src/mapping/*` 历史路径前缀的兼容解析。

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::model::{
    cli::{log_level::LogLevel, Cli},
    config::{global::GlobalConfig, provider::ProviderConfig},
};

use super::{load, load_field_mapping, load_provider_config, resolve_inventory_seed_paths};

#[test]
fn load_provider_config_matches_global_key_case_insensitively() {
    // 验证 global.providers 键名大小写不敏感匹配。
    let mut global = GlobalConfig::default();
    let provider = ProviderConfig {
        default_asset_account: Some("Assets:Broker:Case:Cash".to_string()),
        ..ProviderConfig::default()
    };

    global
        .providers
        .insert("MyProvider".to_string(), provider.clone());

    let (loaded, source_path) =
        load_provider_config(Path::new("__missing__.yml"), "myprovider", &global)
            .expect("global provider context lookup should work");

    assert!(source_path.is_none());
    assert_eq!(loaded.default_asset_account, provider.default_asset_account);
}

#[test]
fn load_normalizes_provider_name_before_resolving_paths() {
    // 验证入口 `load` 会先把 provider 名标准化为小写，再参与路径查找。
    let cli = Cli {
        provider: "WECHAT".to_string(),
        source: PathBuf::from("dummy.csv"),
        config: PathBuf::from("__missing__.yml"),
        global_config: None,
        output: None,
        log_level: LogLevel::Warn,
        quiet: false,
        verbose: false,
        strict: false,
    };

    let loaded =
        load(&cli).expect("uppercase provider name should still resolve config and mapping");
    assert!(loaded.mapping.date.is_some() || loaded.mapping.amount.is_some());
}

#[test]
fn load_provider_config_falls_back_to_categorized_layout() {
    // 验证 provider 配置会优先回退到新分层目录结构。
    let global = GlobalConfig::default();

    let (_provider, source_path) =
        load_provider_config(Path::new("__missing__.yml"), "wechat", &global)
            .expect("provider config should fallback to categorized config path");

    let source_path = source_path.expect("fallback provider config path should be recorded");
    let normalized = source_path.to_string_lossy().replace('\\', "/");
    assert!(
        normalized.ends_with("config/third_party/wechat.yml"),
        "expected categorized config path, got {}",
        normalized
    );
}

#[test]
fn load_field_mapping_falls_back_to_categorized_layout() {
    // 验证 mapping 文件缺省时会命中新分层目录。
    let provider = ProviderConfig::default();

    let mapping = load_field_mapping(
        &provider,
        "wechat",
        Path::new("config/third_party/wechat.yml"),
    )
    .expect("field mapping should fallback to categorized mapping path");
    assert!(mapping.date.is_some() || mapping.amount.is_some());
}

#[test]
fn load_field_mapping_supports_flat_mapping_path_after_categorized_migration() {
    // 验证旧平铺路径 mapping/<provider>.yml 会自动扩展到分层目录。
    let provider = ProviderConfig {
        mapping_file: Some("mapping/wechat.yml".to_string()),
        ..ProviderConfig::default()
    };

    let mapping = load_field_mapping(
        &provider,
        "wechat",
        Path::new("config/third_party/wechat.yml"),
    )
    .expect("flat mapping path should fallback to categorized mapping path");
    assert!(mapping.date.is_some() || mapping.amount.is_some());
}

#[test]
fn resolves_relative_inventory_seed_paths_against_config_base() {
    // 验证相对路径按配置文件目录解析，绝对路径保持原样。
    let mut provider = ProviderConfig {
        inventory_seed_files: vec![
            "transactions/2025/12/galaxy.bean".to_string(),
            "C:/already/absolute.bean".to_string(),
        ],
        ..ProviderConfig::default()
    };

    resolve_inventory_seed_paths(&mut provider, Path::new("config-new/galaxy.yml"));

    let normalized_first = provider.inventory_seed_files[0].replace('\\', "/");
    assert!(normalized_first.ends_with("config-new/transactions/2025/12/galaxy.bean"));
    assert_eq!(
        provider.inventory_seed_files[1],
        "C:/already/absolute.bean".to_string()
    );
}

#[test]
fn keeps_windows_unc_inventory_seed_paths_unchanged() {
    // 验证 Windows UNC 路径会被识别为“有效绝对路径”且不被改写。
    let mut provider = ProviderConfig {
        inventory_seed_files: vec![
            r"\\nas\beancount\inventory\seed.bean".to_string(),
            "transactions/2026/01/current.bean".to_string(),
        ],
        ..ProviderConfig::default()
    };

    resolve_inventory_seed_paths(&mut provider, Path::new("config-new/galaxy.yml"));

    assert_eq!(
        provider.inventory_seed_files[0],
        r"\\nas\beancount\inventory\seed.bean".to_string()
    );
    let normalized_second = provider.inventory_seed_files[1].replace('\\', "/");
    assert!(normalized_second.ends_with("config-new/transactions/2026/01/current.bean"));
}

#[test]
fn load_field_mapping_supports_legacy_src_mapping_prefix() {
    // 构造临时目录，模拟“新目录 + 旧 mapping_file 前缀”的迁移场景。
    let mut temp_root = std::env::temp_dir();
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    temp_root.push(format!(
        "beancount-mapping-compat-{}-{}",
        std::process::id(),
        unique
    ));

    let config_dir = temp_root.join("config-new");
    let mapping_dir = config_dir.join("mapping");
    fs::create_dir_all(&mapping_dir).expect("mapping test directory should be created");

    let mapping_file = mapping_dir.join("yinhe.yml");
    fs::write(
        &mapping_file,
        r#"
date: "成交日期"
amount: "成交金额"
"#,
    )
    .expect("mapping test file should be writable");

    let provider = ProviderConfig {
        mapping_file: Some("src/mapping/yinhe.yml".to_string()),
        ..ProviderConfig::default()
    };

    let mapping = load_field_mapping(&provider, "yinhe", &config_dir.join("galaxy.yml"))
        .expect("legacy src/mapping prefix should fallback to mapping/");
    assert!(mapping.date.is_some());
    assert!(mapping.amount.is_some());

    // 测试结束后尝试清理目录；即使失败也不影响断言结果。
    let _ = fs::remove_dir_all(temp_root);
}
