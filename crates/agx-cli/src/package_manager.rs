use std::process::Command;

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

pub fn install_agent(
    agent: AgentDefinition,
    context: &CliContext,
) -> Result<LifecycleResult, AgxError> {
    if inspection::find_binary_in_path(agent.binary_name).is_some() {
        return Ok(LifecycleResult {
            changed: false,
            install_state: state::get_installed_agent_state(agent.name),
            installed: true,
            message: Some(format!("{} is already installed.", agent.display_name)),
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

    run_external_command(&command)?;
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
    if inspection::find_binary_in_path(agent.binary_name).is_some() {
        return Ok(LifecycleResult {
            changed: false,
            install_state: state::get_installed_agent_state(agent.name),
            installed: true,
            message: Some(format!("{} is already installed.", agent.display_name)),
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

    run_external_command(&command)?;
    state::remove_installed_agent_state(agent.name)?;
    Ok(LifecycleResult {
        changed: true,
        install_state: None,
        installed: false,
        message: None,
    })
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

fn run_external_command(command: &[String]) -> Result<(), AgxError> {
    let Some((program, args)) = command.split_first() else {
        return Err(AgxError::new(
            AgxErrorCode::InvalidArgument,
            "Empty command",
        ));
    };

    let status = Command::new(program).args(args).status().map_err(|error| {
        AgxError::new(
            AgxErrorCode::InstallFailed,
            format!("Failed to run `{}`: {error}", command.join(" ")),
        )
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(AgxError::new(
            AgxErrorCode::InstallFailed,
            format!("Command `{}` exited with {status}.", command.join(" ")),
        ))
    }
}
