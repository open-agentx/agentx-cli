use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};

use crate::errors::{AgxError, AgxErrorCode};

pub fn load_effective_config() -> BTreeMap<String, Value> {
    let mut config = default_config();
    if let Ok(raw) = fs::read_to_string(config_file_path())
        && let Ok(Value::Object(stored)) = serde_json::from_str::<Value>(&raw)
    {
        for (key, value) in stored {
            if is_supported_key(&key) {
                config.insert(key, value);
            }
        }
    }

    config
}

pub fn get_config_value(key: &str) -> Value {
    load_effective_config().remove(key).unwrap_or(Value::Null)
}

pub fn set_config_value(key: &str, raw_value: &str) -> Result<Value, AgxError> {
    let value = normalize_value(key, raw_value)?;
    let mut config = load_effective_config();
    config.insert(key.to_string(), value.clone());
    save_config(&config)?;
    Ok(value)
}

pub fn reset_config() -> Result<BTreeMap<String, Value>, AgxError> {
    let config = default_config();
    save_config(&config)?;
    Ok(config)
}

fn save_config(config: &BTreeMap<String, Value>) -> Result<(), AgxError> {
    let path = config_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AgxError::new(
                AgxErrorCode::InvalidArgument,
                format!("Failed to create config directory: {error}"),
            )
        })?;
    }

    let json = serde_json::to_string_pretty(config).map_err(|error| {
        AgxError::new(
            AgxErrorCode::InvalidArgument,
            format!("Failed to serialize config: {error}"),
        )
    })?;
    fs::write(path, format!("{json}\n")).map_err(|error| {
        AgxError::new(
            AgxErrorCode::InvalidArgument,
            format!("Failed to write config: {error}"),
        )
    })
}

fn default_config() -> BTreeMap<String, Value> {
    BTreeMap::from([
        ("defaultPackageManager".to_string(), json!("bun")),
        ("networkRetries".to_string(), json!(2)),
        ("networkTimeoutMs".to_string(), json!(10_000)),
        ("npmBunUpdateStrategy".to_string(), json!("latest-major")),
        ("selfUpdateChannel".to_string(), json!("stable")),
        ("selfUpdateRegistry".to_string(), Value::Null),
        ("versionCacheTtlHours".to_string(), json!(6)),
    ])
}

fn normalize_value(key: &str, value: &str) -> Result<Value, AgxError> {
    match key {
        "defaultPackageManager" => match value {
            "bun" | "npm" => Ok(json!(value)),
            _ => Err(invalid("defaultPackageManager must be bun or npm")),
        },
        "npmBunUpdateStrategy" => match value {
            "latest-major" | "respect-semver" => Ok(json!(value)),
            _ => Err(invalid(
                "npmBunUpdateStrategy must be latest-major or respect-semver",
            )),
        },
        "selfUpdateChannel" => match value {
            "stable" | "beta" => Ok(json!(value)),
            _ => Err(invalid("selfUpdateChannel must be stable or beta")),
        },
        "selfUpdateRegistry" => {
            if value.starts_with("https://") || value.starts_with("http://") {
                Ok(json!(value.trim_end_matches('/')))
            } else {
                Err(invalid("selfUpdateRegistry must be a valid absolute URL"))
            }
        }
        "networkRetries" | "networkTimeoutMs" | "versionCacheTtlHours" => {
            let parsed = value
                .parse::<u64>()
                .map_err(|_| invalid(format!("{key} must be a positive integer")))?;
            if parsed == 0 {
                return Err(invalid(format!("{key} must be a positive integer")));
            }
            Ok(json!(parsed))
        }
        _ => Err(invalid(format!("Unknown config key: {key}"))),
    }
}

fn is_supported_key(key: &str) -> bool {
    matches!(
        key,
        "defaultPackageManager"
            | "networkRetries"
            | "networkTimeoutMs"
            | "npmBunUpdateStrategy"
            | "selfUpdateChannel"
            | "selfUpdateRegistry"
            | "versionCacheTtlHours"
    )
}

fn invalid(message: impl Into<String>) -> AgxError {
    AgxError::new(AgxErrorCode::InvalidArgument, message)
}

fn config_file_path() -> PathBuf {
    config_dir().join("config.json")
}

fn config_dir() -> PathBuf {
    home_dir().join(".quantex")
}

fn home_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}
