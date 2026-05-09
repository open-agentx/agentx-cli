use std::process::Command;
use std::{fs, path::PathBuf};

use crate::agents::AgentDefinition;
use crate::config;
use crate::context::CliContext;
use crate::errors::{AgxError, AgxErrorCode};
use crate::inspection;
use crate::state::{self, InstalledAgentState};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleResult {
    pub changed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_state: Option<InstalledAgentState>,
    pub installed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResult {
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
}

pub fn install_agent(
    agent: AgentDefinition,
    context: &CliContext,
) -> Result<LifecycleResult, AgxError> {
    if let Some(binary_path) = inspection::find_binary_in_path(agent.binary_name) {
        let install_state = state::get_installed_agent_state(agent.name);
        let inferred_state = install_state
            .clone()
            .or_else(|| infer_existing_install_state(agent, &binary_path));
        let adopted = install_state.is_none() && inferred_state.is_some();
        if adopted && !context.dry_run {
            state::set_installed_agent_state(
                inferred_state
                    .clone()
                    .expect("inferred state should exist when adopted"),
            )?;
        }
        let message = if install_state.is_some() {
            format!("{} is already installed.", agent.display_name)
        } else if adopted {
            format!(
                "AGX is now tracking the existing install of {}.",
                agent.display_name
            )
        } else {
            format!(
                "{} is already installed, but this install is not tracked by AGX.",
                agent.display_name
            )
        };
        return Ok(LifecycleResult {
            changed: adopted,
            install_state: inferred_state,
            installed: true,
            message: Some(message),
        });
    }

    let Some(package) = agent.npm_package else {
        return Err(AgxError::new(
            AgxErrorCode::ManualActionRequired,
            format!(
                "{} does not have a managed npm or Bun package yet.",
                agent.display_name
            ),
        ));
    };

    let install_type = preferred_package_manager();
    let command = install_command(install_type, package);

    if context.dry_run {
        return Ok(LifecycleResult {
            changed: false,
            install_state: Some(installed_state(agent, install_type, package)),
            installed: false,
            message: Some(format!("Dry run: would run `{}`.", command.join(" "))),
        });
    }

    run_external_command(&command, AgxErrorCode::InstallFailed)?;
    let installed_state = installed_state(agent, install_type, package);
    state::set_installed_agent_state(installed_state.clone())?;
    Ok(LifecycleResult {
        changed: true,
        install_state: Some(installed_state),
        installed: true,
        message: None,
    })
}

pub fn ensure_agent(
    agent: AgentDefinition,
    context: &CliContext,
) -> Result<LifecycleResult, AgxError> {
    if let Some(binary_path) = inspection::find_binary_in_path(agent.binary_name) {
        let install_state = state::get_installed_agent_state(agent.name);
        let inferred_state = install_state
            .clone()
            .or_else(|| infer_existing_install_state(agent, &binary_path));
        let adopted = install_state.is_none() && inferred_state.is_some();
        if adopted && !context.dry_run {
            state::set_installed_agent_state(
                inferred_state
                    .clone()
                    .expect("inferred state should exist when adopted"),
            )?;
        }
        let message = if install_state.is_some() {
            format!("{} is already installed.", agent.display_name)
        } else if adopted {
            format!(
                "AGX is now tracking the existing install of {}.",
                agent.display_name
            )
        } else {
            format!(
                "{} is already installed, but this install is not tracked by AGX.",
                agent.display_name
            )
        };
        return Ok(LifecycleResult {
            changed: adopted,
            install_state: inferred_state,
            installed: true,
            message: Some(message),
        });
    }

    install_agent(agent, context)
}

pub fn uninstall_agent(
    agent: AgentDefinition,
    context: &CliContext,
) -> Result<LifecycleResult, AgxError> {
    let Some(installed_state) = state::get_installed_agent_state(agent.name) else {
        return Err(AgxError::new(
            AgxErrorCode::AgentNotInstalled,
            format!("{} is not tracked as installed by AGX.", agent.display_name),
        ));
    };

    let Some(package_name) = installed_state.package_name.as_deref() else {
        return Err(AgxError::new(
            AgxErrorCode::ManualActionRequired,
            format!(
                "{} does not have a managed package recorded.",
                agent.display_name
            ),
        ));
    };

    let command = uninstall_command(&installed_state.install_type, package_name);

    if context.dry_run {
        return Ok(LifecycleResult {
            changed: false,
            install_state: Some(installed_state),
            installed: true,
            message: Some(format!("Dry run: would run `{}`.", command.join(" "))),
        });
    }

    run_external_command(&command, AgxErrorCode::UninstallFailed)?;
    state::remove_installed_agent_state(agent.name)?;
    Ok(LifecycleResult {
        changed: true,
        install_state: None,
        installed: false,
        message: None,
    })
}

pub fn update_agent(
    agent: AgentDefinition,
    installed_state: Option<&InstalledAgentState>,
    installed_version: Option<String>,
    latest_version: Option<String>,
    context: &CliContext,
) -> Result<UpdateResult, AgxError> {
    let strategy = update_strategy(agent, installed_state);

    match strategy {
        "managed" => update_managed_agent(
            agent,
            installed_state,
            installed_version,
            latest_version,
            context,
        ),
        "self-update" => Ok(update_self_updating_agent(
            agent,
            installed_state,
            installed_version,
            latest_version,
            context,
        )),
        _ => Ok(UpdateResult {
            display_name: agent.display_name.to_string(),
            hint: Some(get_update_failure_hint(agent, "manual-hint")),
            installed_version,
            latest_version,
            name: agent.name.to_string(),
            message: Some(format!(
                "{} uses a manually managed install source. Please check for updates manually.",
                agent.display_name
            )),
            resource: None,
            status: "manual-required",
            strategy: Some("manual-hint".to_string()),
        }),
    }
}

pub fn update_agents_by_type(install_type: &str, packages: &[String]) -> Result<(), AgxError> {
    let mut unique_packages = Vec::new();
    for package in packages {
        if !unique_packages.contains(package) {
            unique_packages.push(package.clone());
        }
    }

    if unique_packages.is_empty() {
        return Ok(());
    }

    let command = update_many_command(install_type, &unique_packages)?;
    run_external_command(&command, AgxErrorCode::UpdateFailed)
}

pub fn get_managed_installed_package_version(
    install_type: &str,
    package_name: &str,
) -> Option<String> {
    let env_key = format!(
        "AGX_TEST_MANAGED_VERSION_{}",
        sanitize_env_key(package_name)
    );
    if let Ok(version) = std::env::var(env_key) {
        return Some(version);
    }

    match install_type {
        "npm" => get_npm_installed_package_version(package_name),
        "bun" => get_bun_installed_package_version(package_name),
        _ => None,
    }
}

fn update_managed_agent(
    agent: AgentDefinition,
    installed_state: Option<&InstalledAgentState>,
    installed_version: Option<String>,
    latest_version: Option<String>,
    context: &CliContext,
) -> Result<UpdateResult, AgxError> {
    let (install_type, package_name) = if let Some(installed_state) = installed_state {
        let Some(package_name) = installed_state.package_name.as_deref() else {
            return Ok(UpdateResult {
                display_name: agent.display_name.to_string(),
                hint: None,
                installed_version,
                latest_version,
                name: agent.name.to_string(),
                message: Some("No managed package is recorded for this agent.".to_string()),
                resource: None,
                status: "manual-required",
                strategy: None,
            });
        };
        (installed_state.install_type.as_str(), package_name)
    } else if let Some(package_name) = agent.npm_package {
        (preferred_package_manager(), package_name)
    } else {
        return Ok(UpdateResult {
            display_name: agent.display_name.to_string(),
            hint: Some(get_update_failure_hint(agent, "manual-hint")),
            installed_version,
            latest_version,
            name: agent.name.to_string(),
            message: Some(format!(
                "{} uses a manually managed install source. Please check for updates manually.",
                agent.display_name
            )),
            resource: None,
            status: "manual-required",
            strategy: Some("manual-hint".to_string()),
        });
    };

    let command = update_command(install_type, package_name);
    let strategy = Some(format!("managed/{install_type}"));

    if context.dry_run {
        return Ok(UpdateResult {
            display_name: agent.display_name.to_string(),
            hint: None,
            installed_version,
            latest_version,
            name: agent.name.to_string(),
            message: Some(format!("Dry run: would run `{}`.", command.join(" "))),
            resource: None,
            status: "planned",
            strategy,
        });
    }

    run_external_command(&command, AgxErrorCode::UpdateFailed)?;
    Ok(UpdateResult {
        display_name: agent.display_name.to_string(),
        hint: None,
        installed_version,
        latest_version,
        name: agent.name.to_string(),
        message: None,
        resource: None,
        status: "updated",
        strategy,
    })
}

fn update_self_updating_agent(
    agent: AgentDefinition,
    installed_state: Option<&InstalledAgentState>,
    installed_version: Option<String>,
    latest_version: Option<String>,
    context: &CliContext,
) -> UpdateResult {
    let commands = if let Some(installed_state) = installed_state {
        installed_state.command.as_deref().map_or_else(
            || crate::agents::self_update_commands(agent),
            |command| vec![command],
        )
    } else {
        crate::agents::self_update_commands(agent)
    };

    if commands.is_empty() {
        return UpdateResult {
            display_name: agent.display_name.to_string(),
            hint: Some(get_update_failure_hint(agent, "manual-hint")),
            installed_version,
            latest_version,
            name: agent.name.to_string(),
            message: Some(format!(
                "{} uses a manually managed install source. Please check for updates manually.",
                agent.display_name
            )),
            resource: None,
            status: "manual-required",
            strategy: Some("manual-hint".to_string()),
        };
    }

    if context.dry_run {
        return UpdateResult {
            display_name: agent.display_name.to_string(),
            hint: None,
            installed_version,
            latest_version,
            name: agent.name.to_string(),
            message: Some(format!("Dry run: would run `{}`.", commands[0])),
            resource: None,
            status: "planned",
            strategy: Some("self-update".to_string()),
        };
    }

    for command in commands {
        let parsed = shell_words(command);
        if run_external_command(&parsed, AgxErrorCode::UpdateFailed).is_ok() {
            return UpdateResult {
                display_name: agent.display_name.to_string(),
                hint: None,
                installed_version,
                latest_version,
                name: agent.name.to_string(),
                message: None,
                resource: None,
                status: "updated",
                strategy: Some("self-update".to_string()),
            };
        }
    }

    UpdateResult {
        display_name: agent.display_name.to_string(),
        hint: Some(get_update_failure_hint(agent, "self-update")),
        installed_version,
        latest_version,
        name: agent.name.to_string(),
        message: Some(format!("Failed to update {}.", agent.display_name)),
        resource: None,
        status: "failed",
        strategy: Some("self-update".to_string()),
    }
}

fn preferred_package_manager() -> &'static str {
    let config = config::load_effective_config();
    match config
        .get("defaultPackageManager")
        .and_then(serde_json::Value::as_str)
    {
        Some("npm") => "npm",
        _ => "bun",
    }
}

fn npm_bun_update_strategy() -> &'static str {
    let config = config::load_effective_config();
    match config
        .get("npmBunUpdateStrategy")
        .and_then(serde_json::Value::as_str)
    {
        Some("respect-semver") => "respect-semver",
        _ => "latest-major",
    }
}

fn install_command(install_type: &str, package: &str) -> Vec<String> {
    match install_type {
        "npm" => vec![
            "npm".to_string(),
            "install".to_string(),
            "-g".to_string(),
            package.to_string(),
        ],
        _ => vec![
            "bun".to_string(),
            "add".to_string(),
            "-g".to_string(),
            package.to_string(),
        ],
    }
}

fn update_command(install_type: &str, package: &str) -> Vec<String> {
    let strategy = npm_bun_update_strategy();
    match (install_type, strategy) {
        ("npm", "respect-semver") => vec![
            "npm".to_string(),
            "update".to_string(),
            "-g".to_string(),
            package.to_string(),
        ],
        ("npm", _) => vec![
            "npm".to_string(),
            "install".to_string(),
            "-g".to_string(),
            format!("{package}@latest"),
        ],
        ("bun", "respect-semver") => vec![
            "bun".to_string(),
            "update".to_string(),
            "-g".to_string(),
            package.to_string(),
        ],
        _ => vec![
            "bun".to_string(),
            "update".to_string(),
            "-g".to_string(),
            "--latest".to_string(),
            package.to_string(),
        ],
    }
}

fn update_many_command(install_type: &str, packages: &[String]) -> Result<Vec<String>, AgxError> {
    let strategy = npm_bun_update_strategy();
    match (install_type, strategy) {
        ("npm", "respect-semver") => Ok(std::iter::once("npm".to_string())
            .chain(["update", "-g"].into_iter().map(str::to_string))
            .chain(packages.iter().cloned())
            .collect()),
        ("npm", _) => Ok(std::iter::once("npm".to_string())
            .chain(["install", "-g"].into_iter().map(str::to_string))
            .chain(packages.iter().map(|package| format!("{package}@latest")))
            .collect()),
        ("bun", "respect-semver") => Ok(std::iter::once("bun".to_string())
            .chain(["update", "-g"].into_iter().map(str::to_string))
            .chain(packages.iter().cloned())
            .collect()),
        ("bun", _) => Ok(std::iter::once("bun".to_string())
            .chain(["update", "-g", "--latest"].into_iter().map(str::to_string))
            .chain(packages.iter().cloned())
            .collect()),
        _ => Err(AgxError::new(
            AgxErrorCode::InvalidArgument,
            format!("Unsupported managed update type: {install_type}"),
        )),
    }
}

fn uninstall_command(install_type: &str, package: &str) -> Vec<String> {
    match install_type {
        "npm" => vec![
            "npm".to_string(),
            "uninstall".to_string(),
            "-g".to_string(),
            package.to_string(),
        ],
        _ => vec![
            "bun".to_string(),
            "remove".to_string(),
            "-g".to_string(),
            package.to_string(),
        ],
    }
}

fn installed_state(
    agent: AgentDefinition,
    install_type: &str,
    package: &str,
) -> InstalledAgentState {
    InstalledAgentState {
        agent_name: agent.name.to_string(),
        install_type: install_type.to_string(),
        package_name: Some(package.to_string()),
        package_target_kind: Some("package".to_string()),
        command: None,
    }
}

fn infer_existing_install_state(
    agent: AgentDefinition,
    binary_path: &std::path::Path,
) -> Option<InstalledAgentState> {
    let normalized = binary_path.to_string_lossy().replace('\\', "/");
    if let Some(package_name) = agent.npm_package {
        let install_type =
            if normalized.contains("/.bun/bin/") || normalized.contains("/.bun/install/global/") {
                Some("bun")
            } else if normalized.contains("/node_modules/.bin/")
                || normalized.contains("/node_modules/")
            {
                Some("npm")
            } else {
                None
            };
        if let Some(install_type) = install_type {
            return Some(installed_state(agent, install_type, package_name));
        }
    }

    if agent.npm_package.is_none() {
        return crate::agents::self_update_commands(agent)
            .first()
            .map(|command| InstalledAgentState {
                agent_name: agent.name.to_string(),
                install_type: "script".to_string(),
                package_name: None,
                package_target_kind: None,
                command: Some((*command).to_string()),
            });
    }

    None
}

fn run_external_command(command: &[String], error_code: AgxErrorCode) -> Result<(), AgxError> {
    if let Ok(path) = std::env::var("AGX_TEST_CAPTURE_COMMAND_PATH") {
        let path = PathBuf::from(path);
        let mut existing = fs::read_to_string(&path).unwrap_or_default();
        if !existing.is_empty() && !existing.ends_with('\n') {
            existing.push('\n');
        }
        existing.push_str(&command.join(" "));
        existing.push('\n');
        let _ = fs::write(path, existing);
    }

    if std::env::var("AGX_TEST_ALLOW_EXTERNAL_SUCCESS").as_deref() == Ok("1") {
        if let (Ok(binary_name), Ok(directory)) = (
            std::env::var("AGX_TEST_CREATE_BINARY_NAME"),
            std::env::var("AGX_TEST_CREATE_BINARY_DIR"),
        ) {
            let source = std::env::current_exe().map_err(|error| {
                AgxError::new(
                    error_code,
                    format!("Failed to locate AGX test executable: {error}"),
                )
            })?;
            let extension = if cfg!(windows) { ".exe" } else { "" };
            let target = PathBuf::from(directory).join(format!("{binary_name}{extension}"));
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    AgxError::new(
                        error_code,
                        format!("Failed to prepare AGX test binary directory: {error}"),
                    )
                })?;
            }
            fs::copy(source, target).map_err(|error| {
                AgxError::new(
                    error_code,
                    format!("Failed to create AGX test binary: {error}"),
                )
            })?;
        }
        return Ok(());
    }

    let Some((program, args)) = command.split_first() else {
        return Err(AgxError::new(
            AgxErrorCode::InvalidArgument,
            "Empty command",
        ));
    };

    let output = Command::new(program).args(args).output().map_err(|error| {
        AgxError::new(
            error_code,
            format!("Failed to run `{}`: {error}", command.join(" ")),
        )
    })?;

    if output.status.success() {
        Ok(())
    } else {
        Err(AgxError::new(
            error_code,
            format!(
                "Command `{}` exited with {}.",
                command.join(" "),
                output.status
            ),
        ))
    }
}

fn update_strategy(
    agent: AgentDefinition,
    installed_state: Option<&InstalledAgentState>,
) -> &'static str {
    if installed_state.is_some_and(|state| {
        matches!(
            state.install_type.as_str(),
            "bun" | "npm" | "brew" | "winget"
        )
    }) {
        "managed"
    } else if installed_state
        .and_then(|state| state.command.as_ref())
        .is_some()
        || (installed_state.is_some() && !crate::agents::self_update_commands(agent).is_empty())
    {
        "self-update"
    } else if agent.npm_package.is_some() {
        "managed"
    } else if installed_state
        .and_then(|state| state.command.as_ref())
        .is_some()
        || !crate::agents::self_update_commands(agent).is_empty()
    {
        "self-update"
    } else {
        "manual-hint"
    }
}

fn get_update_failure_hint(agent: AgentDefinition, strategy: &str) -> String {
    if strategy == "self-update"
        && let Some(command) = crate::agents::self_update_commands(agent).first()
    {
        return format!("Try running {command} directly.");
    }

    format!("Check {} for the recommended update path.", agent.homepage)
}

fn shell_words(command: &str) -> Vec<String> {
    command
        .split_whitespace()
        .map(ToString::to_string)
        .collect()
}

fn get_npm_installed_package_version(package_name: &str) -> Option<String> {
    let output = Command::new("npm")
        .args(["list", "-g", "--depth=0", "--json"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    json["dependencies"][package_name]["version"]
        .as_str()
        .map(ToString::to_string)
}

fn get_bun_installed_package_version(package_name: &str) -> Option<String> {
    let output = Command::new("bun").args(["pm", "-g", "ls"]).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let marker = format!("{package_name}@");
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().find_map(|line| {
        let token = line.split_whitespace().last()?;
        token
            .strip_prefix(&marker)
            .filter(|version| !version.is_empty())
            .map(ToString::to_string)
    })
}

fn sanitize_env_key(value: &str) -> String {
    value
        .chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() {
                char.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}
