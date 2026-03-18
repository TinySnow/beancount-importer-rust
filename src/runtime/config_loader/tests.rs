//! 模块说明：配置加载模块测试实现。
//!
//! 文件路径：src/runtime/config_loader/tests.rs。
//! 该文件主要包含单元测试与回归测试。
//! 关键符号：load_provider_config_matches_global_key_case_insensitively、load_normalizes_provider_name_before_resolving_paths、resolves_relative_inventory_seed_paths_against_config_base、load_field_mapping_supports_legacy_src_mapping_prefix。

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::model::{
    cli::{Cli, log_level::LogLevel},
    config::{global::GlobalConfig, provider::ProviderConfig},
};

use super::{load, load_field_mapping, load_provider_config, resolve_inventory_seed_paths};

#[test]
fn load_provider_config_matches_global_key_case_insensitively() {
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
fn resolves_relative_inventory_seed_paths_against_config_base() {
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
fn load_field_mapping_supports_legacy_src_mapping_prefix() {
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

    let _ = fs::remove_dir_all(temp_root);
}
