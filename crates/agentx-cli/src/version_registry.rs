use std::collections::BTreeMap;
use std::fs;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{ETAG, IF_NONE_MATCH, USER_AGENT};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::config;
use crate::context::{CacheMode, CliContext, CliFreshness, FreshnessSource, record_freshness};
use crate::state;

pub const OFFICIAL_NPM_REGISTRY: &str = "https://registry.npmjs.org";
const ISO_8601_MILLIS: &[time::format_description::FormatItem<'static>] = time::macros::format_description!(
    "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z"
);

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct CachedResponseStore {
    entries: BTreeMap<String, CachedResponseEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CachedResponseEntry {
    body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    etag: Option<String>,
    expires_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    fetched_at: Option<u64>,
}

pub fn get_latest_version(
    package_name: &str,
    dist_tag: &str,
    registry: Option<&str>,
    context: &CliContext,
) -> Option<String> {
    let registry = registry
        .and_then(normalize_registry_url)
        .or_else(|| std::env::var("AGX_TEST_PACKAGE_REGISTRY").ok())
        .unwrap_or_else(|| OFFICIAL_NPM_REGISTRY.to_string());
    let url = build_registry_package_version_url(package_name, dist_tag, &registry)?;
    let cache_key = format!("npm:{registry}:{package_name}:{dist_tag}");
    fetch_json_with_cache::<RegistryVersionResponse>(&url, &cache_key, context, None)
        .map(|response| response.version)
}

pub fn fetch_json_with_cache<T: DeserializeOwned>(
    url: &str,
    cache_key: &str,
    context: &CliContext,
    user_agent: Option<&str>,
) -> Option<T> {
    let body = fetch_text_with_cache(url, cache_key, context, user_agent, true)?;
    serde_json::from_str(&body).ok()
}

pub fn normalize_registry_url(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    reqwest::Url::parse(trimmed)
        .ok()
        .map(|url| url.to_string().trim_end_matches('/').to_string())
}

pub fn build_registry_package_version_url(
    package_name: &str,
    dist_tag: &str,
    registry_url: &str,
) -> Option<String> {
    let registry = normalize_registry_url(registry_url)?;
    Some(format!(
        "{registry}/{}/{dist_tag}",
        urlencoding::encode(package_name)
    ))
}

fn fetch_text_with_cache(
    url: &str,
    cache_key: &str,
    context: &CliContext,
    user_agent: Option<&str>,
    validate_json: bool,
) -> Option<String> {
    let settings = network_settings();
    let ttl_ms = settings
        .version_cache_ttl_hours
        .saturating_mul(60 * 60 * 1_000);
    let mut cache = if matches!(context.cache_mode, CacheMode::NoCache) {
        CachedResponseStore::default()
    } else {
        load_response_cache()
    };
    let cached_entry = cache.entries.get(cache_key).cloned();
    let now = current_timestamp_ms();

    if matches!(context.cache_mode, CacheMode::Default)
        && let Some(entry) = &cached_entry
        && entry.expires_at > now
    {
        record_freshness(context, freshness_from_cache(entry, ttl_ms));
        return Some(entry.body.clone());
    }

    let response = fetch_with_retries(
        url,
        settings.network_retries,
        settings.network_timeout_ms,
        if matches!(context.cache_mode, CacheMode::NoCache) {
            None
        } else {
            cached_entry
                .as_ref()
                .and_then(|entry| entry.etag.as_deref())
        },
        user_agent,
    );

    let Some(response) = response else {
        if let Some(entry) = &cached_entry {
            record_freshness(context, freshness_from_cache(entry, ttl_ms));
        }
        return cached_entry.map(|entry| entry.body);
    };

    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        let mut entry = cached_entry?;
        entry.expires_at = now.saturating_add(ttl_ms);
        entry.fetched_at = Some(now);
        if !matches!(context.cache_mode, CacheMode::NoCache) {
            cache.entries.insert(cache_key.to_string(), entry.clone());
            let _ = save_response_cache(&cache);
        }
        record_freshness(context, freshness_from_network(now, entry.expires_at));
        return Some(entry.body);
    }

    if !response.status().is_success() {
        if let Some(entry) = &cached_entry {
            record_freshness(context, freshness_from_cache(entry, ttl_ms));
        }
        return cached_entry.map(|entry| entry.body);
    }

    let etag = response
        .headers()
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string);
    let body = response.text().ok()?;

    if validate_json && serde_json::from_str::<serde_json::Value>(&body).is_err() {
        if let Some(entry) = &cached_entry {
            record_freshness(context, freshness_from_cache(entry, ttl_ms));
        }
        return cached_entry.map(|entry| entry.body);
    }

    if !matches!(context.cache_mode, CacheMode::NoCache) {
        cache.entries.insert(
            cache_key.to_string(),
            CachedResponseEntry {
                body: body.clone(),
                etag,
                expires_at: now.saturating_add(ttl_ms),
                fetched_at: Some(now),
            },
        );
        let _ = save_response_cache(&cache);
    }

    record_freshness(
        context,
        freshness_from_network(now, now.saturating_add(ttl_ms)),
    );

    Some(body)
}

fn freshness_from_cache(entry: &CachedResponseEntry, ttl_ms: u64) -> CliFreshness {
    let fetched_at_ms = entry
        .fetched_at
        .unwrap_or_else(|| entry.expires_at.saturating_sub(ttl_ms));
    CliFreshness {
        fetched_at: timestamp_ms_to_iso8601(fetched_at_ms),
        source: FreshnessSource::Cache,
        stale_after: timestamp_ms_to_iso8601(entry.expires_at),
    }
}

fn freshness_from_network(fetched_at: u64, stale_after: u64) -> CliFreshness {
    CliFreshness {
        fetched_at: timestamp_ms_to_iso8601(fetched_at),
        source: FreshnessSource::Network,
        stale_after: timestamp_ms_to_iso8601(stale_after),
    }
}

fn timestamp_ms_to_iso8601(timestamp_ms: u64) -> String {
    use time::OffsetDateTime;

    let seconds = i64::try_from(timestamp_ms / 1_000).ok();
    let nanos = i64::try_from((timestamp_ms % 1_000) * 1_000_000).ok();

    seconds
        .zip(nanos)
        .and_then(|(seconds, nanos)| {
            OffsetDateTime::from_unix_timestamp(seconds)
                .ok()
                .map(|datetime| datetime + time::Duration::nanoseconds(nanos))
        })
        .and_then(|datetime| datetime.format(ISO_8601_MILLIS).ok())
        .unwrap_or_else(|| "1970-01-01T00:00:00.000Z".to_string())
}

fn fetch_with_retries(
    url: &str,
    retries: u64,
    timeout_ms: u64,
    etag: Option<&str>,
    user_agent: Option<&str>,
) -> Option<reqwest::blocking::Response> {
    for attempt in 0..=retries {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .ok()?;
        let mut request = client.get(url);
        if let Some(etag) = etag {
            request = request.header(IF_NONE_MATCH, etag);
        }
        if let Some(user_agent) = user_agent {
            request = request.header(USER_AGENT, user_agent);
        }

        match request.send() {
            Ok(response) => return Some(response),
            Err(_) if attempt < retries => {}
            Err(_) => return None,
        }
    }

    None
}

fn load_response_cache() -> CachedResponseStore {
    let path = cache_file_path();
    let Ok(raw) = fs::read_to_string(path) else {
        return CachedResponseStore::default();
    };

    serde_json::from_str(&raw).unwrap_or_default()
}

fn save_response_cache(cache: &CachedResponseStore) -> Result<(), std::io::Error> {
    let path = cache_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json =
        serde_json::to_string_pretty(cache).unwrap_or_else(|_| "{\"entries\":{}}".to_string());
    fs::write(path, format!("{json}\n"))
}

fn cache_file_path() -> std::path::PathBuf {
    state::state_file_path().parent().map_or_else(
        || std::path::PathBuf::from("cache").join("versions.json"),
        |parent| parent.join("cache").join("versions.json"),
    )
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

fn network_settings() -> NetworkSettings {
    let config = config::load_effective_config();
    NetworkSettings {
        network_retries: config
            .get("networkRetries")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(2),
        network_timeout_ms: config
            .get("networkTimeoutMs")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(10_000),
        version_cache_ttl_hours: config
            .get("versionCacheTtlHours")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(6),
    }
}

#[derive(Debug)]
struct NetworkSettings {
    network_retries: u64,
    network_timeout_ms: u64,
    version_cache_ttl_hours: u64,
}

#[derive(Debug, Deserialize)]
struct RegistryVersionResponse {
    version: String,
}
