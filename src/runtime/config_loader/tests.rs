//! `config_loader` 模块测试。
//!
//! 文件路径：`src/runtime/config_loader/tests.rs`。
//!
//! 本文件主要覆盖以下回归场景：
//! - 供应商名称大小写归一化与配置回退顺序；
//! - `inventory_seed_files` 的跨平台绝对/相对路径处理；
//! - 去兼容化后的映射路径行为（不再支持旧别名路径）。

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    model::config::{
        defaults::CommonDefaultsConfig, global::GlobalConfig, provider::ProviderConfig,
    },
    runtime::cli::{Cli, log_level::LogLevel},
};

use super::{
    global::load_global_config, load, load_field_mapping, load_provider_config,
    resolve_inventory_seed_paths,
};

#[test]
fn load_provider_config_matches_global_key_case_insensitively() {
    // 验证 global.providers 键名大小写不敏感匹配。
    let mut global = GlobalConfig::default();
    let provider = ProviderConfig {
        defaults: CommonDefaultsConfig {
            asset_account: Some("Assets:Broker:Case:Cash".to_string()),
            ..CommonDefaultsConfig::default()
        },
        ..ProviderConfig::default()
    };

    global
        .providers
        .insert("MyProvider".to_string(), provider.clone());

    let (loaded, source_path) =
        load_provider_config(Path::new("__missing__.yml"), "myprovider", &global)
            .expect("global provider context lookup should work");

    assert!(source_path.is_none());
    assert_eq!(
        loaded.default_asset_account.as_deref(),
        Some("Assets:Broker:Case:Cash")
    );
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
    // 验证 provider 配置会优先回退到分层目录结构。
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
fn load_field_mapping_does_not_support_flat_mapping_legacy_path() {
    // 去兼容化后，旧平铺路径 mapping/<provider>.yml 不再自动扩展到分层目录。
    let provider = ProviderConfig {
        mapping_file: Some("mapping/wechat.yml".to_string()),
        ..ProviderConfig::default()
    };

    let result = load_field_mapping(
        &provider,
        "wechat",
        Path::new("config/third_party/wechat.yml"),
    );
    assert!(
        result.is_err(),
        "legacy flat mapping path should no longer be accepted"
    );
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
fn load_field_mapping_does_not_support_legacy_src_mapping_prefix() {
    // 去兼容化后，src/mapping/* 旧前缀不再自动替换为 mapping/*。
    let provider = ProviderConfig {
        mapping_file: Some("src/mapping/yinhe.yml".to_string()),
        ..ProviderConfig::default()
    };

    let result = load_field_mapping(&provider, "yinhe", &PathBuf::from("config/galaxy.yml"));
    assert!(
        result.is_err(),
        "legacy src/mapping prefix should no longer be accepted"
    );
}

#[test]
fn load_provider_config_normalizes_default_group_fields() {
    let mut temp_root = std::env::temp_dir();
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    temp_root.push(format!(
        "beancount-provider-default-group-{}-{}",
        std::process::id(),
        unique
    ));

    fs::create_dir_all(&temp_root).expect("provider test directory should be created");
    let provider_path = temp_root.join("provider.yml");
    fs::write(
        &provider_path,
        r#"
name: "test"
default:
  asset_account: "Assets:Test:Cash"
  expense_account: "Expenses:Test"
  income_account: "Income:Test"
  currency: "USD"
"#,
    )
    .expect("provider test file should be writable");

    let global = GlobalConfig::default();
    let (provider, source_path) =
        load_provider_config(&provider_path, "test", &global).expect("provider config should load");

    assert_eq!(
        source_path.expect("provider source path should exist"),
        provider_path
    );
    assert_eq!(
        provider.default_asset_account.as_deref(),
        Some("Assets:Test:Cash")
    );
    assert_eq!(
        provider.default_expense_account.as_deref(),
        Some("Expenses:Test")
    );
    assert_eq!(
        provider.default_income_account.as_deref(),
        Some("Income:Test")
    );
    assert_eq!(provider.default_currency.as_deref(), Some("USD"));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn load_global_config_normalizes_default_group_fields() {
    let mut temp_root = std::env::temp_dir();
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    temp_root.push(format!(
        "beancount-global-default-group-{}-{}",
        std::process::id(),
        unique
    ));

    fs::create_dir_all(&temp_root).expect("global test directory should be created");
    let global_path = temp_root.join("global.yml");
    fs::write(
        &global_path,
        r#"
default:
  asset_account: "Assets:Global:Cash"
  expense_account: "Expenses:Global"
  income_account: "Income:Global"
  currency: "USD"
"#,
    )
    .expect("global test file should be writable");

    let (global, source_path) =
        load_global_config(Some(global_path.as_path())).expect("global config should load");

    assert_eq!(
        source_path.expect("global source path should exist"),
        global_path
    );
    assert_eq!(
        global.default_asset_account.as_deref(),
        Some("Assets:Global:Cash")
    );
    assert_eq!(
        global.default_expense_account.as_deref(),
        Some("Expenses:Global")
    );
    assert_eq!(
        global.default_income_account.as_deref(),
        Some("Income:Global")
    );
    assert_eq!(global.default_currency, "USD");

    let _ = fs::remove_dir_all(temp_root);
}
