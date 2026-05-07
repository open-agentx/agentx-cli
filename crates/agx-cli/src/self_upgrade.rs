use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config;
use crate::context::CliContext;
use crate::errors::{AgxError, AgxErrorCode};
use crate::state;
use crate::version_registry;

const AGX_PACKAGE_NAME: &str = "agxctl";
const REPOSITORY_URL: &str = env!("CARGO_PKG_REPOSITORY");

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<SelfUpdateChannel>,
    pub command: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_version: Option<String>,
    pub dry_run: bool,
    pub install_source: InstallSourceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub package_name: &'static str,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_version: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SelfUpdateChannel {
    Stable,
    Beta,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallSourceKind {
    Bun,
    Npm,
    SourceBuild,
    Standalone,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfInspection {
    pub can_auto_update: bool,
    pub current_version: String,
    pub executable_path: String,
    pub install_source: InstallSourceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managed_registry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_upgrade_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_latest_version: Option<String>,
    pub update_channel: SelfUpdateChannel,
}

#[derive(Debug, Clone)]
struct BinaryReleaseManifest {
    version: String,
    assets: Vec<BinaryReleaseAsset>,
}

#[derive(Debug, Clone)]
struct BinaryReleaseAsset {
    arch: String,
    checksum: String,
    download_url: String,
    name: String,
    platform: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBinaryReleaseManifest {
    assets: Vec<RawBinaryReleaseAsset>,
    version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBinaryReleaseAsset {
    arch: String,
    #[serde(default)]
    checksum: Option<String>,
    #[serde(default)]
    download_url: Option<String>,
    name: String,
    #[serde(default)]
    os: Option<String>,
    #[serde(default)]
    platform: Option<String>,
    #[serde(default)]
    sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseSummary {
    assets: Vec<GitHubReleaseAsset>,
    prerelease: bool,
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseAsset {
    browser_download_url: String,
    name: String,
}

pub fn inspect_self_with_context(
    requested_channel: Option<SelfUpdateChannel>,
    context: &CliContext,
) -> SelfInspection {
    let executable = effective_executable_path();
    let install_source = detect_install_source(&executable);
    let channel = resolve_channel(requested_channel);
    let latest_version = resolve_latest_version_for(install_source, channel, &executable, context);

    SelfInspection {
        can_auto_update: can_auto_update(install_source),
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        executable_path: executable.to_string_lossy().into_owned(),
        install_source,
        latest_version,
        managed_registry: if matches!(
            install_source,
            InstallSourceKind::Bun | InstallSourceKind::Npm
        ) {
            resolve_registry_override()
        } else {
            None
        },
        recommended_upgrade_command: if can_auto_update(install_source) {
            Some(match channel {
                SelfUpdateChannel::Stable => "agx upgrade".to_string(),
                SelfUpdateChannel::Beta => "agx upgrade --channel beta".to_string(),
            })
        } else {
            None
        },
        upstream_latest_version: if matches!(
            install_source,
            InstallSourceKind::Bun | InstallSourceKind::Npm
        ) {
            resolve_upstream_latest_version(channel, context)
        } else {
            None
        },
        update_channel: channel,
    }
}

pub fn upgrade_self(
    context: &CliContext,
    requested_channel: Option<SelfUpdateChannel>,
    check: bool,
) -> Result<UpgradeData, AgxError> {
    let executable = effective_executable_path();
    let install_source = detect_install_source(&executable);
    let channel = resolve_channel(requested_channel);
    let current_version = Some(env!("CARGO_PKG_VERSION").to_string());
    let latest_version = resolve_latest_version_for(install_source, channel, &executable, context);

    if check {
        let Some(latest_version) = latest_version else {
            return Err(AgxError::new(
                AgxErrorCode::NetworkError,
                "Unable to determine the latest AGX version.",
            ));
        };

        let status = if is_version_newer(&latest_version, env!("CARGO_PKG_VERSION")) {
            "update-available"
        } else {
            "up-to-date"
        };

        return Ok(UpgradeData {
            channel: Some(channel),
            command: Vec::new(),
            current_version,
            dry_run: context.dry_run,
            install_source,
            latest_version: Some(latest_version),
            recovery_hint: None,
            message: None,
            package_name: AGX_PACKAGE_NAME,
            status,
            verified_version: None,
        });
    }

    if let Some(latest_version) = latest_version.as_deref()
        && !is_version_newer(latest_version, env!("CARGO_PKG_VERSION"))
    {
        return Ok(UpgradeData {
            channel: Some(channel),
            command: Vec::new(),
            current_version,
            dry_run: context.dry_run,
            install_source,
            latest_version: Some(latest_version.to_string()),
            recovery_hint: None,
            message: Some(
                if is_version_older(latest_version, env!("CARGO_PKG_VERSION")) {
                    format!(
                        "Selected registry reported {latest_version}, which is older than the current AGX version {}; AGX will not downgrade.",
                        env!("CARGO_PKG_VERSION")
                    )
                } else {
                    "AGX is already up to date.".to_string()
                },
            ),
            package_name: AGX_PACKAGE_NAME,
            status: "up-to-date",
            verified_version: None,
        });
    }

    match install_source {
        InstallSourceKind::Npm => {
            upgrade_managed(context, "npm", channel, current_version, latest_version)
        }
        InstallSourceKind::Bun => {
            upgrade_managed(context, "bun", channel, current_version, latest_version)
        }
        InstallSourceKind::Standalone => upgrade_standalone(
            context,
            channel,
            current_version,
            latest_version,
            &executable,
        ),
        InstallSourceKind::SourceBuild => Err(AgxError::new(
            AgxErrorCode::ManualActionRequired,
            "This AGX binary appears to be a source build. Rebuild it with `cargo build --release`.",
        )),
        InstallSourceKind::Unknown => Err(AgxError::new(
            AgxErrorCode::ManualActionRequired,
            "AGX could not determine its install source. Reinstall through npm, Bun, or a standalone release.",
        )),
    }
}

fn upgrade_managed(
    context: &CliContext,
    program: &'static str,
    channel: SelfUpdateChannel,
    current_version: Option<String>,
    latest_version: Option<String>,
) -> Result<UpgradeData, AgxError> {
    let version_tag = if channel == SelfUpdateChannel::Beta {
        "beta"
    } else {
        "latest"
    };
    let package_spec = format!("{AGX_PACKAGE_NAME}@{version_tag}");
    let args = if program == "npm" {
        vec![
            "install".to_string(),
            "-g".to_string(),
            package_spec.clone(),
        ]
    } else {
        vec!["add".to_string(), "-g".to_string(), package_spec.clone()]
    };

    let command: Vec<String> = std::iter::once(program.to_string()).chain(args).collect();

    if context.dry_run {
        let status = latest_version
            .as_deref()
            .filter(|latest| is_version_newer(latest, env!("CARGO_PKG_VERSION")))
            .map_or("planned", |_| "update-available");
        return Ok(UpgradeData {
            channel: Some(channel),
            command,
            current_version,
            dry_run: true,
            install_source: if program == "npm" {
                InstallSourceKind::Npm
            } else {
                InstallSourceKind::Bun
            },
            latest_version,
            recovery_hint: None,
            message: Some(format!(
                "Dry run: would run managed self-upgrade through {program}."
            )),
            package_name: AGX_PACKAGE_NAME,
            status,
            verified_version: None,
        });
    }

    run_external_command(
        &command,
        if program == "npm" {
            InstallSourceKind::Npm
        } else {
            InstallSourceKind::Bun
        },
        channel,
    )?;
    let verified_version = verify_current_version();
    Ok(UpgradeData {
        channel: Some(channel),
        command,
        current_version,
        dry_run: false,
        install_source: if program == "npm" {
            InstallSourceKind::Npm
        } else {
            InstallSourceKind::Bun
        },
        latest_version,
        recovery_hint: None,
        message: None,
        package_name: AGX_PACKAGE_NAME,
        status: "upgraded",
        verified_version,
    })
}

fn upgrade_standalone(
    context: &CliContext,
    channel: SelfUpdateChannel,
    current_version: Option<String>,
    latest_version: Option<String>,
    executable: &Path,
) -> Result<UpgradeData, AgxError> {
    let manifest = fetch_binary_release_manifest(channel)?;
    let Some(asset) = resolve_binary_release_asset(&manifest, executable) else {
        return Err(AgxError::new(
            AgxErrorCode::ManualActionRequired,
            format!(
                "No standalone AGX release asset is available for {}-{}.",
                std::env::consts::OS,
                std::env::consts::ARCH
            ),
        ));
    };

    if context.dry_run {
        return Ok(UpgradeData {
            channel: Some(channel),
            command: vec![asset.download_url.clone()],
            current_version,
            dry_run: true,
            install_source: InstallSourceKind::Standalone,
            latest_version,
            recovery_hint: Some(format!(
                "download and replace the binary from {}",
                asset.download_url
            )),
            message: Some(format!(
                "Dry run: would download {} and replace the standalone AGX binary.",
                asset.name
            )),
            package_name: AGX_PACKAGE_NAME,
            status: "planned",
            verified_version: None,
        });
    }

    perform_standalone_upgrade(asset, executable, manifest.version.as_str())?;
    let verified_version = verify_binary_version(executable);
    Ok(UpgradeData {
        channel: Some(channel),
        command: vec![asset.download_url.clone()],
        current_version,
        dry_run: false,
        install_source: InstallSourceKind::Standalone,
        latest_version,
        recovery_hint: None,
        message: None,
        package_name: AGX_PACKAGE_NAME,
        status: "upgraded",
        verified_version,
    })
}

pub fn get_recovery_hint(
    install_source: InstallSourceKind,
    channel: SelfUpdateChannel,
) -> Option<String> {
    let version_tag = if channel == SelfUpdateChannel::Beta {
        "beta"
    } else {
        "latest"
    };

    match install_source {
        InstallSourceKind::Bun => Some(format!("bun add -g {AGX_PACKAGE_NAME}@{version_tag}")),
        InstallSourceKind::Npm => Some(format!("npm install -g {AGX_PACKAGE_NAME}@{version_tag}")),
        InstallSourceKind::Standalone => fetch_binary_release_manifest(channel)
            .ok()
            .and_then(|manifest| {
                resolve_binary_release_asset(&manifest, &effective_executable_path()).map(|asset| {
                    format!(
                        "download and replace the binary from {}",
                        asset.download_url
                    )
                })
            })
            .or_else(|| {
                Some(
                    "download and replace the AGX binary from the latest release assets"
                        .to_string(),
                )
            }),
        InstallSourceKind::SourceBuild => Some("cargo build --release".to_string()),
        InstallSourceKind::Unknown => None,
    }
}

fn detect_install_source(executable: &Path) -> InstallSourceKind {
    if let Some(recorded) = state::load_state().self_state.install_source {
        return match recorded.as_str() {
            "bun" => InstallSourceKind::Bun,
            "npm" => InstallSourceKind::Npm,
            "standalone" => InstallSourceKind::Standalone,
            "source-build" => InstallSourceKind::SourceBuild,
            _ => InstallSourceKind::Unknown,
        };
    }

    let executable_text = executable.to_string_lossy().replace('\\', "/");
    if executable_text.contains("/node_modules/") || executable_text.contains("/npm/") {
        InstallSourceKind::Npm
    } else if executable_text.contains("/.bun/") || executable_text.contains("/bun/") {
        InstallSourceKind::Bun
    } else if executable_text.contains("/target/debug/")
        || executable_text.contains("/target/release/")
    {
        InstallSourceKind::SourceBuild
    } else if executable
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem.eq_ignore_ascii_case("agx"))
    {
        InstallSourceKind::Standalone
    } else {
        InstallSourceKind::Unknown
    }
}

fn effective_executable_path() -> PathBuf {
    std::env::var_os("AGX_TEST_SELF_EXECUTABLE_PATH")
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
        .unwrap_or_else(|| PathBuf::from("agx"))
}

fn run_external_command(
    command: &[String],
    install_source: InstallSourceKind,
    channel: SelfUpdateChannel,
) -> Result<(), AgxError> {
    if let Ok(mode) = std::env::var("AGX_TEST_UPGRADE_FAILURE") {
        let message = match mode.as_str() {
            "locked" => "Another agx upgrade is already running.",
            "permission" => "Failed to replace the current AGX binary.",
            "npm" => "Failed to update agxctl through npm.",
            "bun" => "Failed to update agxctl through Bun.",
            _ => "Failed to upgrade AGX.",
        };
        let recovery_hint = if mode == "locked" {
            Some(
                "another agx upgrade is already running; wait for it to finish and retry"
                    .to_string(),
            )
        } else {
            get_recovery_hint(install_source, channel)
        };
        return Err(AgxError::new(
            if mode == "locked" {
                AgxErrorCode::ResourceLocked
            } else {
                AgxErrorCode::UpgradeFailed
            },
            recovery_hint.map_or_else(
                || message.to_string(),
                |hint| format!("{message} Next step: {hint}"),
            ),
        ));
    }

    let Some((program, args)) = command.split_first() else {
        return Err(AgxError::new(
            AgxErrorCode::InvalidArgument,
            "Empty command",
        ));
    };

    let status = Command::new(program).args(args).status().map_err(|error| {
        AgxError::new(
            AgxErrorCode::UpgradeFailed,
            format!("Failed to run `{}`: {error}", command.join(" ")),
        )
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(AgxError::new(
            AgxErrorCode::UpgradeFailed,
            format!("Command `{}` exited with {status}.", command.join(" ")),
        ))
    }
}

fn verify_current_version() -> Option<String> {
    let executable = effective_executable_path();
    verify_binary_version(&executable)
}

fn verify_binary_version(executable: &Path) -> Option<String> {
    let output = Command::new(executable).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .last()
        .map(ToString::to_string)
}

fn resolve_channel(requested_channel: Option<SelfUpdateChannel>) -> SelfUpdateChannel {
    if let Some(requested_channel) = requested_channel {
        return requested_channel;
    }

    match config::get_config_value("selfUpdateChannel").as_str() {
        Some("beta") => SelfUpdateChannel::Beta,
        _ => SelfUpdateChannel::Stable,
    }
}

fn resolve_latest_version_for(
    install_source: InstallSourceKind,
    channel: SelfUpdateChannel,
    executable: &Path,
    context: &CliContext,
) -> Option<String> {
    if matches!(install_source, InstallSourceKind::Standalone) {
        return fetch_binary_release_manifest(channel)
            .ok()
            .and_then(|manifest| {
                if resolve_binary_release_asset(&manifest, executable).is_some() {
                    Some(manifest.version)
                } else {
                    None
                }
            });
    }

    resolve_latest_version(channel, context)
}

fn resolve_latest_version(channel: SelfUpdateChannel, context: &CliContext) -> Option<String> {
    if let Ok(version) = std::env::var("AGX_TEST_LATEST_VERSION") {
        return Some(version);
    }

    let registry = resolve_registry_override()
        .unwrap_or_else(|| version_registry::OFFICIAL_NPM_REGISTRY.to_string());
    let dist_tag = if channel == SelfUpdateChannel::Beta {
        "beta"
    } else {
        "latest"
    };
    version_registry::get_latest_version(AGX_PACKAGE_NAME, dist_tag, Some(&registry), context)
}

fn resolve_upstream_latest_version(
    channel: SelfUpdateChannel,
    context: &CliContext,
) -> Option<String> {
    if let Ok(version) = std::env::var("AGX_TEST_UPSTREAM_LATEST_VERSION") {
        return Some(version);
    }

    let dist_tag = if channel == SelfUpdateChannel::Beta {
        "beta"
    } else {
        "latest"
    };
    version_registry::get_latest_version(
        AGX_PACKAGE_NAME,
        dist_tag,
        Some(version_registry::OFFICIAL_NPM_REGISTRY),
        context,
    )
}

fn resolve_registry_override() -> Option<String> {
    config::get_config_value("selfUpdateRegistry")
        .as_str()
        .map(|value| value.trim_end_matches('/').to_string())
}

fn is_version_newer(candidate: &str, current: &str) -> bool {
    match (
        semver::Version::parse(candidate),
        semver::Version::parse(current),
    ) {
        (Ok(candidate), Ok(current)) => candidate > current,
        _ => candidate != current,
    }
}

pub fn is_version_older(candidate: &str, current: &str) -> bool {
    match (
        semver::Version::parse(candidate),
        semver::Version::parse(current),
    ) {
        (Ok(candidate), Ok(current)) => candidate < current,
        _ => false,
    }
}

fn can_auto_update(install_source: InstallSourceKind) -> bool {
    matches!(
        install_source,
        InstallSourceKind::Bun | InstallSourceKind::Npm | InstallSourceKind::Standalone
    )
}

fn fetch_binary_release_manifest(
    channel: SelfUpdateChannel,
) -> Result<BinaryReleaseManifest, AgxError> {
    let manifest_source = std::env::var("AGX_TEST_STANDALONE_MANIFEST_PATH").ok();
    let raw = if let Some(path) = manifest_source {
        fs::read_to_string(path).map_err(|error| {
            AgxError::new(
                AgxErrorCode::NetworkError,
                format!("Failed to read standalone release manifest: {error}"),
            )
        })?
    } else {
        let url = resolve_binary_release_manifest_url(channel)?;
        Client::builder()
            .build()
            .map_err(|error| AgxError::new(AgxErrorCode::NetworkError, error.to_string()))?
            .get(url)
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|error| {
                AgxError::new(
                    AgxErrorCode::NetworkError,
                    format!("Failed to download standalone release manifest: {error}"),
                )
            })?
            .text()
            .map_err(|error| {
                AgxError::new(
                    AgxErrorCode::NetworkError,
                    format!("Failed to read standalone release manifest: {error}"),
                )
            })?
    };

    let parsed: RawBinaryReleaseManifest = serde_json::from_str(&raw).map_err(|error| {
        AgxError::new(
            AgxErrorCode::InvalidArgument,
            format!("Failed to parse standalone release manifest: {error}"),
        )
    })?;

    let assets = parsed
        .assets
        .into_iter()
        .map(normalize_binary_release_asset)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(BinaryReleaseManifest {
        version: parsed.version,
        assets,
    })
}

fn normalize_binary_release_asset(
    raw: RawBinaryReleaseAsset,
) -> Result<BinaryReleaseAsset, AgxError> {
    let checksum = raw.checksum.or(raw.sha256).ok_or_else(|| {
        AgxError::new(
            AgxErrorCode::InvalidArgument,
            format!(
                "Standalone release asset {} is missing checksum metadata.",
                raw.name
            ),
        )
    })?;
    let platform = raw.platform.or(raw.os).ok_or_else(|| {
        AgxError::new(
            AgxErrorCode::InvalidArgument,
            format!(
                "Standalone release asset {} is missing platform metadata.",
                raw.name
            ),
        )
    })?;
    let download_url = raw
        .download_url
        .unwrap_or_else(|| format!("{}/releases/latest/download/{}", REPOSITORY_URL, raw.name));

    Ok(BinaryReleaseAsset {
        arch: normalize_release_arch(&raw.arch).to_string(),
        checksum: checksum.trim().to_lowercase(),
        download_url,
        name: raw.name,
        platform: normalize_release_platform(&platform).to_string(),
    })
}

fn resolve_binary_release_manifest_url(channel: SelfUpdateChannel) -> Result<String, AgxError> {
    if let Ok(url) = std::env::var("AGX_TEST_STANDALONE_MANIFEST_URL") {
        return Ok(url);
    }

    if channel == SelfUpdateChannel::Stable {
        return Ok(format!(
            "{REPOSITORY_URL}/releases/latest/download/manifest.json"
        ));
    }

    fetch_github_release_summary(channel).and_then(|release| {
        release
            .assets
            .into_iter()
            .find(|asset| asset.name == "manifest.json")
            .map(|asset| asset.browser_download_url)
            .ok_or_else(|| {
                AgxError::new(
                    AgxErrorCode::NetworkError,
                    format!(
                        "No manifest.json asset was found for the {channel:?} release channel."
                    ),
                )
            })
    })
}

fn fetch_github_release_summary(
    channel: SelfUpdateChannel,
) -> Result<GitHubReleaseSummary, AgxError> {
    let Some(repository_slug) = repository_slug() else {
        return Err(AgxError::new(
            AgxErrorCode::NetworkError,
            "Failed to resolve the GitHub repository slug for AGX releases.",
        ));
    };

    let releases_url =
        format!("https://api.github.com/repos/{repository_slug}/releases?per_page=20");
    let releases = Client::builder()
        .build()
        .map_err(|error| AgxError::new(AgxErrorCode::NetworkError, error.to_string()))?
        .get(releases_url)
        .header(reqwest::header::USER_AGENT, "agxctl-self-upgrade")
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| {
            AgxError::new(
                AgxErrorCode::NetworkError,
                format!("Failed to query GitHub releases: {error}"),
            )
        })?
        .json::<Vec<GitHubReleaseSummary>>()
        .map_err(|error| {
            AgxError::new(
                AgxErrorCode::NetworkError,
                format!("Failed to parse GitHub releases: {error}"),
            )
        })?;

    releases
        .into_iter()
        .find(|release| {
            if channel == SelfUpdateChannel::Beta {
                release.prerelease
            } else {
                !release.prerelease
            }
        })
        .ok_or_else(|| {
            AgxError::new(
                AgxErrorCode::NetworkError,
                format!("No GitHub release was found for the {channel:?} channel."),
            )
        })
}

fn repository_slug() -> Option<&'static str> {
    REPOSITORY_URL
        .split("github.com/")
        .nth(1)
        .map(|slug| slug.trim_end_matches('/'))
}

fn resolve_binary_release_asset<'a>(
    manifest: &'a BinaryReleaseManifest,
    executable: &Path,
) -> Option<&'a BinaryReleaseAsset> {
    let current_asset_name = current_binary_asset_name(executable)?;
    let expected_platform = platform_release_name()?;
    let expected_arch = arch_release_name()?;

    manifest.assets.iter().find(|asset| {
        asset.name == current_asset_name
            && asset.platform == expected_platform
            && asset.arch == expected_arch
    })
}

fn current_binary_asset_name(_executable: &Path) -> Option<String> {
    let platform = platform_release_name()?;
    let arch = arch_release_name()?;
    let extension = if platform == "win32" { ".exe" } else { "" };
    Some(format!("agx-{platform}-{arch}{extension}"))
}

fn perform_standalone_upgrade(
    asset: &BinaryReleaseAsset,
    executable: &Path,
    expected_version: &str,
) -> Result<(), AgxError> {
    let bytes = download_standalone_asset_bytes(asset)?;
    let actual_checksum = format!("{:x}", Sha256::digest(&bytes));
    if actual_checksum != asset.checksum {
        return Err(AgxError::new(
            AgxErrorCode::UpgradeFailed,
            format!(
                "Checksum mismatch for {}. Expected {}, got {}.",
                asset.download_url, asset.checksum, actual_checksum
            ),
        ));
    }

    if std::env::var("AGX_TEST_SKIP_STANDALONE_REPLACE").as_deref() == Ok("1") {
        return Ok(());
    }

    let parent = executable.parent().unwrap_or_else(|| Path::new("."));
    let temp_dir = parent.join(".agx-upgrade");
    fs::create_dir_all(&temp_dir).map_err(|error| {
        AgxError::new(
            AgxErrorCode::UpgradeFailed,
            format!("Failed to prepare standalone upgrade directory: {error}"),
        )
    })?;
    let temp_path = temp_dir.join(
        executable
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("agx"),
    );
    let backup_path = executable.with_extension(format!(
        "{}bak",
        executable
            .extension()
            .and_then(|ext| ext.to_str())
            .map_or(String::new(), |ext| format!("{ext}."))
    ));

    fs::write(&temp_path, &bytes).map_err(|error| {
        AgxError::new(
            AgxErrorCode::UpgradeFailed,
            format!("Failed to stage standalone upgrade binary: {error}"),
        )
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mode = fs::metadata(executable)
            .map(|metadata| metadata.permissions().mode())
            .unwrap_or(0o755);
        let _ = fs::set_permissions(&temp_path, fs::Permissions::from_mode(mode));
    }

    let running_executable = std::env::current_exe().ok();
    let same_as_running = running_executable
        .as_ref()
        .is_some_and(|path| path == executable);

    if cfg!(windows) && same_as_running {
        schedule_windows_replacement(
            &temp_path,
            executable,
            &backup_path,
            &temp_dir,
            expected_version,
        )?;
        return Ok(());
    }

    let _ = fs::remove_file(&backup_path);
    fs::rename(executable, &backup_path).map_err(|error| {
        AgxError::new(
            AgxErrorCode::UpgradeFailed,
            format!("Failed to back up current standalone AGX binary: {error}"),
        )
    })?;
    if let Err(error) = fs::rename(&temp_path, executable) {
        let _ = fs::rename(&backup_path, executable);
        return Err(AgxError::new(
            AgxErrorCode::UpgradeFailed,
            format!("Failed to replace standalone AGX binary: {error}"),
        ));
    }

    if std::env::var("AGX_TEST_SKIP_STANDALONE_VERIFY").as_deref() != Ok("1") {
        let verified =
            verify_binary_version(executable).is_some_and(|version| version == expected_version);
        if !verified {
            let _ = fs::remove_file(executable);
            let _ = fs::rename(&backup_path, executable);
            return Err(AgxError::new(
                AgxErrorCode::UpgradeFailed,
                "The upgraded standalone AGX binary failed version verification.",
            ));
        }
    }

    let _ = fs::remove_file(&backup_path);
    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

fn platform_release_name() -> Option<&'static str> {
    match std::env::consts::OS {
        "windows" => Some("win32"),
        "macos" => Some("darwin"),
        "linux" => Some("linux"),
        _ => None,
    }
}

fn arch_release_name() -> Option<&'static str> {
    match std::env::consts::ARCH {
        "x86_64" => Some("x64"),
        "aarch64" => Some("arm64"),
        _ => None,
    }
}

fn normalize_release_platform(platform: &str) -> &str {
    match platform {
        "windows" => "win32",
        other => other,
    }
}

fn normalize_release_arch(arch: &str) -> &str {
    match arch {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    }
}

fn download_standalone_asset_bytes(asset: &BinaryReleaseAsset) -> Result<Vec<u8>, AgxError> {
    if let Ok(path) = std::env::var("AGX_TEST_STANDALONE_DOWNLOAD_PATH") {
        return fs::read(path).map_err(|error| {
            AgxError::new(
                AgxErrorCode::UpgradeFailed,
                format!("Failed to read standalone upgrade test asset: {error}"),
            )
        });
    }

    Client::builder()
        .build()
        .map_err(|error| AgxError::new(AgxErrorCode::NetworkError, error.to_string()))?
        .get(&asset.download_url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| {
            AgxError::new(
                AgxErrorCode::UpgradeFailed,
                format!("Failed to download standalone AGX binary: {error}"),
            )
        })?
        .bytes()
        .map(|bytes| bytes.to_vec())
        .map_err(|error| {
            AgxError::new(
                AgxErrorCode::UpgradeFailed,
                format!("Failed to read standalone AGX binary payload: {error}"),
            )
        })
}

fn schedule_windows_replacement(
    temp_path: &Path,
    executable: &Path,
    backup_path: &Path,
    temp_dir: &Path,
    expected_version: &str,
) -> Result<(), AgxError> {
    let pid = std::process::id();
    let command = format!(
        "$pidToWait = {pid}; \
$tempPath = '{}'; \
$targetPath = '{}'; \
$backupPath = '{}'; \
$tempDir = '{}'; \
$expectedVersion = '{}'; \
while (Get-Process -Id $pidToWait -ErrorAction SilentlyContinue) {{ Start-Sleep -Milliseconds 200 }}; \
if (Test-Path -LiteralPath $backupPath) {{ Remove-Item -LiteralPath $backupPath -Force -ErrorAction SilentlyContinue }}; \
Move-Item -LiteralPath $targetPath -Destination $backupPath -Force; \
Move-Item -LiteralPath $tempPath -Destination $targetPath -Force; \
$output = & $targetPath --version 2>$null; \
if ($LASTEXITCODE -ne 0 -or ($output -notmatch [regex]::Escape($expectedVersion))) {{ \
  Remove-Item -LiteralPath $targetPath -Force -ErrorAction SilentlyContinue; \
  Move-Item -LiteralPath $backupPath -Destination $targetPath -Force; \
  exit 1 \
}}; \
Remove-Item -LiteralPath $backupPath -Force -ErrorAction SilentlyContinue; \
Remove-Item -LiteralPath $tempDir -Force -Recurse -ErrorAction SilentlyContinue",
        escape_powershell_string(&temp_path.to_string_lossy()),
        escape_powershell_string(&executable.to_string_lossy()),
        escape_powershell_string(&backup_path.to_string_lossy()),
        escape_powershell_string(&temp_dir.to_string_lossy()),
        escape_powershell_string(expected_version),
    );

    Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-Command",
            command.as_str(),
        ])
        .spawn()
        .map_err(|error| {
            AgxError::new(
                AgxErrorCode::UpgradeFailed,
                format!("Failed to schedule Windows standalone AGX replacement: {error}"),
            )
        })?;

    Ok(())
}

fn escape_powershell_string(value: &str) -> String {
    value.replace('\'', "''")
}
