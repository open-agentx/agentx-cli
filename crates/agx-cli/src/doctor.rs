use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::agents;
use crate::context::CliContext;
use crate::inspection;
use crate::self_upgrade::{self, SelfInspection};
use crate::state::{self, AgxState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorData {
    pub agents: Vec<DoctorAgent>,
    pub checks: Vec<DoctorCheck>,
    pub install_source: InstallSource,
    pub installers: Installers,
    pub issues: Vec<DoctorIssue>,
    pub ok: bool,
    pub paths: DoctorPaths,
    #[serde(rename = "self")]
    pub self_status: SelfStatus,
    pub summary: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorAgent {
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    pub lifecycle: &'static str,
    pub outdated: bool,
    pub source_label: &'static str,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct Installers {
    pub brew: bool,
    pub bun: bool,
    pub npm: bool,
    pub winget: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfStatus {
    pub can_auto_update: bool,
    pub current_version: String,
    pub install_source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    pub outdated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_hint: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorIssue {
    pub blocking: bool,
    pub category: &'static str,
    pub code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs_ref: Option<&'static str>,
    pub message: String,
    pub severity: &'static str,
    pub subject: IssueSubject,
    pub suggested_action: &'static str,
    pub suggested_commands: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueSubject {
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

pub fn run_doctor(context: &CliContext) -> DoctorData {
    let executable = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("agx"));
    let state_file = state::state_file_path();
    let config_file = state_file.parent().map_or_else(
        || PathBuf::from("config.json"),
        |parent| parent.join("config.json"),
    );
    let self_inspection = self_upgrade::inspect_self_with_context(None, context);
    let install_source = inspect_install_source(&executable);
    let installers = Installers {
        brew: inspection::find_binary_in_path("brew").is_some(),
        bun: inspection::find_binary_in_path("bun").is_some(),
        npm: inspection::find_binary_in_path("npm").is_some(),
        winget: inspection::find_binary_in_path("winget").is_some(),
    };

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
    let agents = inspected_agents();
    let issues = doctor_issues(&installers, &self_inspection, &agents);

    DoctorData {
        agents,
        checks,
        install_source,
        installers,
        issues,
        ok,
        paths: DoctorPaths {
            config_file: config_file.to_string_lossy().into_owned(),
            executable: executable.to_string_lossy().into_owned(),
            state_file: state_file.to_string_lossy().into_owned(),
        },
        self_status: SelfStatus {
            can_auto_update: self_inspection.can_auto_update,
            current_version: self_inspection.current_version.clone(),
            install_source: install_source_label(self_inspection.install_source),
            latest_version: self_inspection.latest_version.clone(),
            outdated: self_outdated(&self_inspection),
            recovery_hint: if self_outdated(&self_inspection) {
                self_upgrade::get_recovery_hint(
                    self_inspection.install_source,
                    self_inspection.update_channel,
                )
            } else {
                None
            },
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

fn inspected_agents() -> Vec<DoctorAgent> {
    agents::all_agents()
        .iter()
        .filter_map(|agent| {
            let inspection = inspection::inspect_agent(*agent, &default_doctor_context());
            if !inspection.installed {
                return None;
            }

            let installed_state = state::get_installed_agent_state(agent.name);
            let lifecycle = if installed_state.is_some() {
                "managed"
            } else {
                "unmanaged"
            };
            let source_label = if let Some(installed_state) = &installed_state {
                if let Some(package_name) = installed_state.package_name.as_deref() {
                    if installed_state.install_type == "bun"
                        || installed_state.install_type == "npm"
                    {
                        Box::leak(
                            format!(
                                "managed via {} ({package_name})",
                                installed_state.install_type
                            )
                            .into_boxed_str(),
                        )
                    } else {
                        "managed"
                    }
                } else {
                    "managed"
                }
            } else {
                "detected in PATH"
            };

            Some(DoctorAgent {
                display_name: agent.display_name.to_string(),
                installed_version: inspection.installed_version,
                latest_version: inspection.latest_version,
                lifecycle,
                outdated: false,
                source_label,
            })
        })
        .collect()
}

#[allow(clippy::too_many_lines)]
fn doctor_issues(
    installers: &Installers,
    self_inspection: &SelfInspection,
    agents: &[DoctorAgent],
) -> Vec<DoctorIssue> {
    let mut issues = Vec::new();

    if !installers.bun && !installers.npm && !installers.brew && !installers.winget {
        issues.push(DoctorIssue {
            blocking: true,
            category: "installers",
            code: "NO_MANAGED_INSTALLER",
            docs_ref: Some("docs/runbooks/quantex-troubleshooting.md"),
            message: "No managed installer found. Install bun, npm, brew, or winget before relying on managed lifecycle operations.".to_string(),
            severity: "warning",
            subject: IssueSubject {
                kind: "system",
                name: None,
            },
            suggested_action: "restore-managed-installer",
            suggested_commands: Vec::new(),
        });
    }

    if matches!(
        self_inspection.install_source,
        self_upgrade::InstallSourceKind::Bun
    ) && !installers.bun
        || matches!(
            self_inspection.install_source,
            self_upgrade::InstallSourceKind::Npm
        ) && !installers.npm
    {
        issues.push(DoctorIssue {
            blocking: true,
            category: "self",
            code: "SELF_INSTALLER_MISSING",
            docs_ref: Some("docs/runbooks/release-and-self-upgrade-debugging.md"),
            message: format!(
                "AGX is tracked as a {} install, but that installer is not available in PATH.",
                install_source_label(self_inspection.install_source)
            ),
            severity: "warning",
            subject: IssueSubject {
                kind: "self",
                name: Some("agx".to_string()),
            },
            suggested_action: "restore-self-installer",
            suggested_commands: self_upgrade::get_recovery_hint(
                self_inspection.install_source,
                self_inspection.update_channel,
            )
            .into_iter()
            .collect(),
        });
    }

    if !self_inspection.can_auto_update {
        issues.push(DoctorIssue {
            blocking: false,
            category: "self",
            code: "SELF_AUTO_UPDATE_UNAVAILABLE",
            docs_ref: Some("docs/runbooks/release-and-self-upgrade-debugging.md"),
            message: format!(
                "AGX cannot auto-update from install source \"{}\". Reinstall via bun, npm, or the standalone binary if you want `agx upgrade` support.",
                install_source_label(self_inspection.install_source)
            ),
            severity: "warning",
            subject: IssueSubject {
                kind: "self",
                name: Some("agx".to_string()),
            },
            suggested_action: "reinstall-self-with-auto-update-source",
            suggested_commands: Vec::new(),
        });
    }

    if self_outdated(self_inspection) {
        issues.push(DoctorIssue {
            blocking: false,
            category: "self",
            code: "SELF_UPDATE_AVAILABLE",
            docs_ref: Some("docs/runbooks/release-and-self-upgrade-debugging.md"),
            message: format!(
                "AGX {} is behind {}.",
                self_inspection.current_version,
                self_inspection.latest_version.clone().unwrap_or_default()
            ),
            severity: "warning",
            subject: IssueSubject {
                kind: "self",
                name: Some("agx".to_string()),
            },
            suggested_action: "run-self-upgrade",
            suggested_commands: self_inspection
                .recommended_upgrade_command
                .clone()
                .into_iter()
                .collect(),
        });
    }

    for agent in agents {
        if agent.lifecycle == "unmanaged" {
            issues.push(DoctorIssue {
                blocking: false,
                category: "agent",
                code: "AGENT_UNTRACKED_IN_PATH",
                docs_ref: Some("docs/runbooks/quantex-troubleshooting.md"),
                message: format!(
                    "{} is available in PATH but not tracked as a managed AGX install.",
                    agent.display_name
                ),
                severity: "warning",
                subject: IssueSubject {
                    kind: "agent",
                    name: Some(
                        agents::all_agents()
                            .iter()
                            .find(|candidate| candidate.display_name == agent.display_name)
                            .map_or_else(String::new, |candidate| candidate.name.to_string()),
                    ),
                },
                suggested_action: "inspect-agent-install-source",
                suggested_commands: agents::all_agents()
                    .iter()
                    .find(|candidate| candidate.display_name == agent.display_name)
                    .map(|candidate| {
                        vec![
                            format!("agx inspect {} --json", candidate.name),
                            format!("agx install {}", candidate.name),
                        ]
                    })
                    .unwrap_or_default(),
            });
        }
    }

    issues
}

fn self_outdated(self_inspection: &SelfInspection) -> bool {
    self_inspection
        .latest_version
        .as_ref()
        .is_some_and(|latest| {
            semver::Version::parse(latest)
                .ok()
                .zip(semver::Version::parse(&self_inspection.current_version).ok())
                .is_some_and(|(latest, current)| latest > current)
        })
}

fn install_source_label(install_source: self_upgrade::InstallSourceKind) -> &'static str {
    match install_source {
        self_upgrade::InstallSourceKind::Bun => "bun",
        self_upgrade::InstallSourceKind::Npm => "npm",
        self_upgrade::InstallSourceKind::SourceBuild => "source",
        self_upgrade::InstallSourceKind::Standalone => "binary",
        self_upgrade::InstallSourceKind::Unknown => "unknown",
    }
}

fn default_doctor_context() -> CliContext {
    CliContext {
        assume_yes: false,
        cache_mode: crate::context::CacheMode::Default,
        color_mode: crate::context::ColorMode::Never,
        dry_run: false,
        idempotency_key: None,
        interactive: false,
        log_level: crate::context::LogLevel::Silent,
        output_mode: crate::context::OutputMode::Json,
        quiet: true,
        run_id: "doctor-inspection".to_string(),
        timeout_ms: None,
    }
}
