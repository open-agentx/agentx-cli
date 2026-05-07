use std::path::{Path, PathBuf};
use std::process::Command;

use reqwest::blocking::Client;
use serde::Serialize;

use crate::config;
use crate::context::CliContext;
use crate::errors::{AgxError, AgxErrorCode};
use crate::state;

const AGX_PACKAGE_NAME: &str = "agxctl";
const OFFICIAL_NPM_REGISTRY: &str = "https://registry.npmjs.org";

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
    pub recommended_upgrade_command: Option<String>,
    pub update_channel: SelfUpdateChannel,
}

pub fn inspect_self(requested_channel: Option<SelfUpdateChannel>) -> SelfInspection {
    let executable = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("agx"));
    let install_source = detect_install_source(&executable);
    let channel = resolve_channel(requested_channel);

    SelfInspection {
        can_auto_update: can_auto_update(install_source),
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        executable_path: executable.to_string_lossy().into_owned(),
        install_source,
        latest_version: resolve_latest_version(channel),
        recommended_upgrade_command: if can_auto_update(install_source) {
            Some(match channel {
                SelfUpdateChannel::Stable => "agx upgrade".to_string(),
                SelfUpdateChannel::Beta => "agx upgrade --channel beta".to_string(),
            })
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
    let executable = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("agx"));
    let install_source = detect_install_source(&executable);
    let channel = resolve_channel(requested_channel);
    let current_version = Some(env!("CARGO_PKG_VERSION").to_string());
    let latest_version = resolve_latest_version(channel);

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
            message: None,
            package_name: AGX_PACKAGE_NAME,
            status,
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
        InstallSourceKind::Standalone => Err(AgxError::new(
            AgxErrorCode::ManualActionRequired,
            "Standalone self-upgrade requires release manifest and checksum metadata.",
        )),
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
            message: Some(format!(
                "Dry run: would run managed self-upgrade through {program}."
            )),
            package_name: AGX_PACKAGE_NAME,
            status: "planned",
            verified_version: None,
        });
    }

    run_external_command(&command)?;
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
        InstallSourceKind::Standalone => {
            Some("download and replace the AGX binary from the latest release assets".to_string())
        }
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

fn run_external_command(command: &[String]) -> Result<(), AgxError> {
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
    let executable = std::env::current_exe().ok()?;
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

fn resolve_latest_version(channel: SelfUpdateChannel) -> Option<String> {
    if let Ok(version) = std::env::var("AGX_TEST_LATEST_VERSION") {
        return Some(version);
    }

    let registry = config::get_config_value("selfUpdateRegistry")
        .as_str()
        .unwrap_or(OFFICIAL_NPM_REGISTRY)
        .trim_end_matches('/')
        .to_string();
    let dist_tag = if channel == SelfUpdateChannel::Beta {
        "beta"
    } else {
        "latest"
    };
    let url = format!("{registry}/{AGX_PACKAGE_NAME}");
    let response = Client::builder().build().ok()?.get(url).send().ok()?;
    let payload = response.json::<serde_json::Value>().ok()?;
    payload["dist-tags"][dist_tag]
        .as_str()
        .map(ToString::to_string)
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

fn can_auto_update(install_source: InstallSourceKind) -> bool {
    matches!(
        install_source,
        InstallSourceKind::Bun | InstallSourceKind::Npm | InstallSourceKind::Standalone
    )
}
