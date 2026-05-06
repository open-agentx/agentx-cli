use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::context::CliContext;
use crate::inspection;
use crate::state::{self, AgxState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorData {
    pub checks: Vec<DoctorCheck>,
    pub install_source: InstallSource,
    pub ok: bool,
    pub paths: DoctorPaths,
    pub summary: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorCheck {
    pub name: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_hint: Option<String>,
    pub status: CheckStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckStatus {
    Ok,
    Warn,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallSource {
    pub kind: InstallSourceKind,
    pub confidence: &'static str,
    pub executable: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallSourceKind {
    Bun,
    Npm,
    SourceBuild,
    Standalone,
    Unknown,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorPaths {
    pub config_file: String,
    pub executable: String,
    pub state_file: String,
}

pub fn run_doctor(_context: &CliContext) -> DoctorData {
    let executable = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("agx"));
    let state_file = state::state_file_path();
    let config_file = state_file.parent().map_or_else(
        || PathBuf::from("config.json"),
        |parent| parent.join("config.json"),
    );
    let install_source = inspect_install_source(&executable);

    let checks = vec![
        executable_check(&executable),
        installer_check("bun"),
        installer_check("npm"),
        json_file_check::<AgxState>("state", &state_file),
        json_file_check::<serde_json::Value>("config", &config_file),
        lock_check("state-lock", &state_file),
        lock_check("self-upgrade", &state_file),
    ];
    let ok = checks
        .iter()
        .all(|check| matches!(check.status, CheckStatus::Ok));

    DoctorData {
        checks,
        install_source,
        ok,
        paths: DoctorPaths {
            config_file: config_file.to_string_lossy().into_owned(),
            executable: executable.to_string_lossy().into_owned(),
            state_file: state_file.to_string_lossy().into_owned(),
        },
        summary: if ok {
            "AGX runtime checks passed."
        } else {
            "AGX runtime checks completed with warnings."
        },
    }
}

fn inspect_install_source(executable: &Path) -> InstallSource {
    let recorded = state::load_state().self_state.install_source;
    if let Some(recorded) = recorded.as_deref() {
        return InstallSource {
            kind: match recorded {
                "bun" => InstallSourceKind::Bun,
                "npm" => InstallSourceKind::Npm,
                "standalone" => InstallSourceKind::Standalone,
                "source-build" => InstallSourceKind::SourceBuild,
                _ => InstallSourceKind::Unknown,
            },
            confidence: "recorded",
            executable: executable.to_string_lossy().into_owned(),
            recorded: Some(recorded.to_string()),
        };
    }

    let executable_text = executable.to_string_lossy().replace('\\', "/");
    let (kind, confidence) =
        if executable_text.contains("/node_modules/") || executable_text.contains("/npm/") {
            (InstallSourceKind::Npm, "heuristic")
        } else if executable_text.contains("/.bun/") || executable_text.contains("/bun/") {
            (InstallSourceKind::Bun, "heuristic")
        } else if executable_text.contains("/target/debug/")
            || executable_text.contains("/target/release/")
        {
            (InstallSourceKind::SourceBuild, "heuristic")
        } else if executable
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem.eq_ignore_ascii_case("agx"))
        {
            (InstallSourceKind::Standalone, "heuristic")
        } else {
            (InstallSourceKind::Unknown, "low")
        };

    InstallSource {
        kind,
        confidence,
        executable: executable.to_string_lossy().into_owned(),
        recorded: None,
    }
}

fn executable_check(executable: &Path) -> DoctorCheck {
    if executable.is_file() {
        DoctorCheck {
            name: "executable",
            detail: Some(executable.to_string_lossy().into_owned()),
            recovery_hint: None,
            status: CheckStatus::Ok,
        }
    } else {
        DoctorCheck {
            name: "executable",
            detail: Some(executable.to_string_lossy().into_owned()),
            recovery_hint: Some(
                "Reinstall AGX from its original distribution channel.".to_string(),
            ),
            status: CheckStatus::Warn,
        }
    }
}

fn installer_check(binary_name: &'static str) -> DoctorCheck {
    if inspection::find_binary_in_path(binary_name).is_some() {
        DoctorCheck {
            name: binary_name,
            detail: Some("available on PATH".to_string()),
            recovery_hint: None,
            status: CheckStatus::Ok,
        }
    } else {
        DoctorCheck {
            name: binary_name,
            detail: Some("not found on PATH".to_string()),
            recovery_hint: Some(format!(
                "Install {binary_name} if you want AGX to manage npm/Bun distributed agents through that channel."
            )),
            status: CheckStatus::Warn,
        }
    }
}

fn json_file_check<T>(name: &'static str, path: &Path) -> DoctorCheck
where
    T: serde::de::DeserializeOwned,
{
    if !path.exists() {
        return DoctorCheck {
            name,
            detail: Some(format!("{} does not exist yet", path.to_string_lossy())),
            recovery_hint: None,
            status: CheckStatus::Ok,
        };
    }

    match fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<T>(strip_json_bom(&raw)).ok())
    {
        Some(_) => DoctorCheck {
            name,
            detail: Some(format!("{} is valid JSON", path.to_string_lossy())),
            recovery_hint: None,
            status: CheckStatus::Ok,
        },
        None => DoctorCheck {
            name,
            detail: Some(format!("{} could not be parsed", path.to_string_lossy())),
            recovery_hint: Some("Move the file aside or repair it as valid JSON.".to_string()),
            status: CheckStatus::Warn,
        },
    }
}

fn lock_check(name: &'static str, state_file: &Path) -> DoctorCheck {
    let lock_path = state_file.parent().map_or_else(
        || PathBuf::from(format!("{name}.lock")),
        |parent| parent.join(format!("{name}.lock")),
    );
    if lock_path.exists() {
        DoctorCheck {
            name,
            detail: Some(format!("{} exists", lock_path.to_string_lossy())),
            recovery_hint: Some(
                "If no AGX process is running, remove the stale lock file.".to_string(),
            ),
            status: CheckStatus::Warn,
        }
    } else {
        DoctorCheck {
            name,
            detail: Some("not locked".to_string()),
            recovery_hint: None,
            status: CheckStatus::Ok,
        }
    }
}

fn strip_json_bom(raw: &str) -> &str {
    raw.strip_prefix('\u{feff}').unwrap_or(raw)
}
