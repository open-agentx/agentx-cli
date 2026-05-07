use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::agents::AgentDefinition;
use crate::cli::InstallPolicyArg;
use crate::context::CliContext;
use crate::errors::{AgxError, AgxErrorCode};
use crate::{inspection, package_manager};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecResult {
    pub agent: ExecAgent,
    pub execution: ExecExecution,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecExecution {
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_path: Option<String>,
    pub command: Vec<String>,
    pub dry_run: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<u8>,
    pub install_policy: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_guidance: Option<ExecInstallGuidance>,
    pub installed_after: bool,
    pub installed_before: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecAgent {
    pub display_name: &'static str,
    pub name: &'static str,
}

#[derive(Debug, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExecInstallGuidance {
    pub docs_ref: &'static str,
    pub install_methods: Vec<ExecInstallMethod>,
    pub suggested_action: &'static str,
    pub suggested_ensure_command: String,
    pub suggested_exec_command: String,
}

#[derive(Debug, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExecInstallMethod {
    pub command: String,
    pub label: &'static str,
    #[serde(rename = "type")]
    pub method_type: &'static str,
}

pub fn execute_agent(
    agent: AgentDefinition,
    args: &[String],
    install_policy: InstallPolicyArg,
    context: &CliContext,
) -> Result<ExecResult, AgxError> {
    let installed_before = inspection::find_binary_in_path(agent.binary_name).is_some();

    if context.dry_run {
        let needs_install = !installed_before || matches!(install_policy, InstallPolicyArg::Always);
        let command = build_display_command(agent.binary_name, args);
        return Ok(ExecResult {
            agent: exec_agent(agent),
            execution: ExecExecution {
                args: args.to_vec(),
                binary_path: inspection::find_binary_in_path(agent.binary_name)
                    .map(|path| path.to_string_lossy().into_owned()),
                command,
                dry_run: true,
                exit_code: None,
                install_policy: install_policy_label(install_policy),
                install_guidance: (!installed_before).then(|| install_guidance(agent, args)),
                installed_after: installed_before,
                installed_before,
                message: Some(if needs_install {
                    format!(
                        "Dry run: would ensure {} is installed, then execute it.",
                        agent.display_name
                    )
                } else {
                    format!("Dry run: would execute {}.", agent.display_name)
                }),
                stderr: None,
                stdout: None,
            },
        });
    }

    if matches!(install_policy, InstallPolicyArg::Always)
        || (!installed_before && matches!(install_policy, InstallPolicyArg::IfMissing))
    {
        package_manager::ensure_agent(agent, context).map_err(|error| {
            if matches!(error.code, AgxErrorCode::InstallFailed) {
                AgxError::new(
                    error.code,
                    format!(
                        "Failed to install {}. {}",
                        agent.display_name, error.message
                    ),
                )
            } else {
                error
            }
        })?;
    }

    let Some(binary_path) = inspection::find_binary_in_path(agent.binary_name) else {
        return Err(AgxError::new(
            AgxErrorCode::AgentNotInstalled,
            format!(
                "{} is not installed. Run `agx ensure {}` or retry with `--install-policy if-missing`.",
                agent.display_name, agent.name
            ),
        ));
    };

    let command = build_display_command(&binary_path.to_string_lossy(), args);
    let output = run_agent_command(&binary_path, args, context.timeout_ms).map_err(|error| {
        if matches!(error.code, AgxErrorCode::InvalidArgument)
            && error.message.contains("Failed to execute agent")
        {
            AgxError::new(
                error.code,
                format!("Failed to launch {}. {}", agent.display_name, error.message),
            )
        } else {
            error
        }
    })?;
    Ok(ExecResult {
        agent: exec_agent(agent),
        execution: ExecExecution {
            args: args.to_vec(),
            binary_path: Some(binary_path.to_string_lossy().into_owned()),
            command,
            dry_run: false,
            exit_code: Some(output.exit_code),
            install_policy: install_policy_label(install_policy),
            install_guidance: None,
            installed_after: true,
            installed_before,
            message: None,
            stderr: non_empty_output(output.stderr),
            stdout: non_empty_output(output.stdout),
        },
    })
}

fn run_agent_command(
    binary_path: &PathBuf,
    args: &[String],
    timeout_ms: Option<u64>,
) -> Result<AgentCommandOutput, AgxError> {
    if let Ok(mode) = std::env::var("AGX_TEST_EXEC_MODE") {
        return match mode.as_str() {
            "timeout" => Err(AgxError::new(
                AgxErrorCode::Timeout,
                format!(
                    "Agent execution timed out after {}ms.",
                    timeout_ms.unwrap_or_default()
                ),
            )),
            "cancelled" => Err(AgxError::new(
                AgxErrorCode::Cancelled,
                "Agent execution was cancelled.",
            )),
            "spawn-fail" => Err(AgxError::new(
                AgxErrorCode::InvalidArgument,
                "Failed to execute agent: synthetic spawn failure",
            )),
            _ => Ok(AgentCommandOutput {
                exit_code: 0,
                stderr: String::new(),
                stdout: String::new(),
            }),
        };
    }

    if let Some(timeout_ms) = timeout_ms {
        return run_agent_command_with_timeout(binary_path, args, timeout_ms);
    }

    let output = Command::new(binary_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| {
            AgxError::new(
                AgxErrorCode::InvalidArgument,
                format!("Failed to execute agent: {error}"),
            )
        })?;

    Ok(AgentCommandOutput::from_output(output))
}

fn run_agent_command_with_timeout(
    binary_path: &PathBuf,
    args: &[String],
    timeout_ms: u64,
) -> Result<AgentCommandOutput, AgxError> {
    let mut child = Command::new(binary_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            AgxError::new(
                AgxErrorCode::InvalidArgument,
                format!("Failed to execute agent: {error}"),
            )
        })?;

    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        if child
            .try_wait()
            .map_err(|error| AgxError::new(AgxErrorCode::InvalidArgument, error.to_string()))?
            .is_some()
        {
            let output = child.wait_with_output().map_err(|error| {
                AgxError::new(
                    AgxErrorCode::InvalidArgument,
                    format!("Failed to collect agent output: {error}"),
                )
            })?;
            return Ok(AgentCommandOutput::from_output(output));
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AgxError::new(
                AgxErrorCode::Timeout,
                format!("Agent execution timed out after {timeout_ms}ms."),
            ));
        }

        std::thread::sleep(Duration::from_millis(25));
    }
}

struct AgentCommandOutput {
    exit_code: u8,
    stderr: String,
    stdout: String,
}

impl AgentCommandOutput {
    fn from_output(output: std::process::Output) -> Self {
        let std::process::Output {
            status,
            stdout,
            stderr,
        } = output;

        Self {
            exit_code: status
                .code()
                .and_then(|code| u8::try_from(code).ok())
                .unwrap_or(1),
            stderr: String::from_utf8_lossy(&stderr).to_string(),
            stdout: String::from_utf8_lossy(&stdout).to_string(),
        }
    }
}

fn exec_agent(agent: AgentDefinition) -> ExecAgent {
    ExecAgent {
        display_name: agent.display_name,
        name: agent.name,
    }
}

fn install_policy_label(install_policy: InstallPolicyArg) -> &'static str {
    match install_policy {
        InstallPolicyArg::Never => "never",
        InstallPolicyArg::IfMissing => "if-missing",
        InstallPolicyArg::Always => "always",
    }
}

fn build_display_command(program: &str, args: &[String]) -> Vec<String> {
    std::iter::once(program.to_string())
        .chain(args.iter().cloned())
        .collect()
}

fn non_empty_output(output: String) -> Option<String> {
    if output.is_empty() {
        None
    } else {
        Some(output)
    }
}

pub fn install_guidance(agent: AgentDefinition, args: &[String]) -> ExecInstallGuidance {
    let install_methods = agent.npm_package.map_or_else(Vec::new, |package| {
        vec![
            ExecInstallMethod {
                command: format!("bun add -g {package}"),
                label: "bun",
                method_type: "bun",
            },
            ExecInstallMethod {
                command: format!("npm install -g {package}"),
                label: "npm",
                method_type: "npm",
            },
        ]
    });

    ExecInstallGuidance {
        docs_ref: "openspec/changes/rewrite-quantex-cli-as-agx-rust/tasks.md",
        install_methods,
        suggested_action: "rerun-with-install-policy",
        suggested_ensure_command: format!("agx ensure {}", agent.name),
        suggested_exec_command: std::iter::once("agx".to_string())
            .chain([
                "exec".to_string(),
                agent.name.to_string(),
                "--install".to_string(),
                "if-missing".to_string(),
                "--".to_string(),
            ])
            .chain(args.iter().cloned())
            .collect::<Vec<_>>()
            .join(" "),
    }
}
