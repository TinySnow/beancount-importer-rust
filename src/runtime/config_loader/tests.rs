use std::path::{Path, PathBuf};

use crate::model::{
    cli::{Cli, log_level::LogLevel},
    config::{global::GlobalConfig, provider::ProviderConfig},
};

use super::{load, load_provider_config, resolve_inventory_seed_paths};

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
