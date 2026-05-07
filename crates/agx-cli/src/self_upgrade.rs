use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::context::CliContext;
use crate::errors::{AgxError, AgxErrorCode};
use crate::state;

const AGX_PACKAGE_NAME: &str = "agxctl";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeData {
    pub command: Vec<String>,
    pub dry_run: bool,
    pub install_source: InstallSourceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub package_name: &'static str,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_version: Option<String>,
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

pub fn upgrade_self(context: &CliContext) -> Result<UpgradeData, AgxError> {
    let executable = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("agx"));
    let install_source = detect_install_source(&executable);
    match install_source {
        InstallSourceKind::Npm => {
            upgrade_managed(context, "npm", &["install", "-g", AGX_PACKAGE_NAME])
        }
        InstallSourceKind::Bun => upgrade_managed(context, "bun", &["add", "-g", AGX_PACKAGE_NAME]),
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
    args: &[&'static str],
) -> Result<UpgradeData, AgxError> {
    let command: Vec<String> = std::iter::once(program.to_string())
        .chain(args.iter().map(|arg| (*arg).to_string()))
        .collect();

    if context.dry_run {
        return Ok(UpgradeData {
            command,
            dry_run: true,
            install_source: if program == "npm" {
                InstallSourceKind::Npm
            } else {
                InstallSourceKind::Bun
            },
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
        command,
        dry_run: false,
        install_source: if program == "npm" {
            InstallSourceKind::Npm
        } else {
            InstallSourceKind::Bun
        },
        message: None,
        package_name: AGX_PACKAGE_NAME,
        status: "upgraded",
        verified_version,
    })
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
