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
    #[serde(skip_serializing)]
    pub exit_code: u8,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecExecution {
    pub args: Vec<String>,
    pub install_policy: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_guidance: Option<ExecInstallGuidance>,
    pub installed: bool,
    pub interactive: bool,
    pub launched: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecAgent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_name: Option<&'static str>,
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

#[allow(clippy::too_many_lines)]
pub fn execute_agent(
    agent: AgentDefinition,
    args: &[String],
    install_policy: InstallPolicyArg,
    context: &CliContext,
) -> Result<ExecResult, AgxError> {
    let installed_before = inspection::find_binary_in_path(agent.binary_name).is_some();
    let interactive =
        context.interactive && matches!(context.output_mode, crate::context::OutputMode::Human);
    let should_prompt_install =
        !installed_before && matches!(install_policy, InstallPolicyArg::Prompt);

    if context.dry_run {
        return Ok(ExecResult {
            agent: exec_agent(agent),
            execution: ExecExecution {
                args: args.to_vec(),
                install_policy: install_policy_label(install_policy),
                install_guidance: (!installed_before).then(|| install_guidance(agent, args)),
                installed: true,
                interactive,
                launched: false,
            },
            exit_code: 0,
        });
    }

    if should_prompt_install {
        if !interactive && !context.assume_yes {
            return Err(AgxError::new(
                AgxErrorCode::InteractionRequired,
                format!(
                    "{} is not installed and interactive installation is disabled.",
                    agent.display_name
                ),
            ));
        }

        if interactive && !context.assume_yes && !confirm_exec_install(agent)? {
            return Err(AgxError::new(
                AgxErrorCode::Cancelled,
                format!("Installation cancelled for {}.", agent.display_name),
            ));
        }
    }

    if matches!(install_policy, InstallPolicyArg::Always)
        || (!installed_before
            && matches!(
                install_policy,
                InstallPolicyArg::IfMissing | InstallPolicyArg::Prompt
            ))
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

    let output =
        run_agent_command(&binary_path, args, context.timeout_ms, context).map_err(|error| {
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
            install_policy: install_policy_label(install_policy),
            install_guidance: None,
            installed: true,
            interactive,
            launched: true,
        },
        exit_code: output.exit_code,
    })
}

fn run_agent_command(
    binary_path: &PathBuf,
    args: &[String],
    timeout_ms: Option<u64>,
    context: &CliContext,
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
            _ => Ok(AgentCommandOutput { exit_code: 0 }),
        };
    }

    if let Some(timeout_ms) = timeout_ms {
        return run_agent_command_with_timeout(
            binary_path,
            args,
            timeout_ms,
            matches!(context.output_mode, crate::context::OutputMode::Human),
        );
    }

    if matches!(context.output_mode, crate::context::OutputMode::Human) {
        let status = Command::new(binary_path)
            .args(args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|error| {
                AgxError::new(
                    AgxErrorCode::InvalidArgument,
                    format!("Failed to execute agent: {error}"),
                )
            })?;

        return Ok(AgentCommandOutput {
            exit_code: status
                .code()
                .and_then(|code| u8::try_from(code).ok())
                .unwrap_or(1),
        });
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

    Ok(AgentCommandOutput::from_output(&output))
}

fn run_agent_command_with_timeout(
    binary_path: &PathBuf,
    args: &[String],
    timeout_ms: u64,
    inherit_stdio: bool,
) -> Result<AgentCommandOutput, AgxError> {
    let mut command = Command::new(binary_path);
    command.args(args);
    if inherit_stdio {
        command
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
    } else {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
    }

    let mut child = command.spawn().map_err(|error| {
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
            if inherit_stdio {
                let status = child.wait().map_err(|error| {
                    AgxError::new(
                        AgxErrorCode::InvalidArgument,
                        format!("Failed to collect agent status: {error}"),
                    )
                })?;
                return Ok(AgentCommandOutput {
                    exit_code: status
                        .code()
                        .and_then(|code| u8::try_from(code).ok())
                        .unwrap_or(1),
                });
            }

            let output = child.wait_with_output().map_err(|error| {
                AgxError::new(
                    AgxErrorCode::InvalidArgument,
                    format!("Failed to collect agent output: {error}"),
                )
            })?;
            return Ok(AgentCommandOutput::from_output(&output));
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
}

impl AgentCommandOutput {
    fn from_output(output: &std::process::Output) -> Self {
        Self {
            exit_code: output
                .status
                .code()
                .and_then(|code| u8::try_from(code).ok())
                .unwrap_or(1),
        }
    }
}

fn exec_agent(agent: AgentDefinition) -> ExecAgent {
    ExecAgent {
        binary_name: Some(agent.binary_name),
        display_name: agent.display_name,
        name: agent.name,
    }
}

fn install_policy_label(install_policy: InstallPolicyArg) -> &'static str {
    match install_policy {
        InstallPolicyArg::Prompt => "prompt",
        InstallPolicyArg::Never => "never",
        InstallPolicyArg::IfMissing => "if-missing",
        InstallPolicyArg::Always => "always",
    }
}

fn confirm_exec_install(agent: AgentDefinition) -> Result<bool, AgxError> {
    eprintln!(
        "{} is not installed. Install it now? [y/N]",
        agent.display_name
    );

    if let Ok(answer) = std::env::var("AGX_TEST_PROMPT_RESPONSE") {
        return Ok(matches!(
            answer.trim().to_ascii_lowercase().as_str(),
            "y" | "yes"
        ));
    }

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|error| AgxError::new(AgxErrorCode::Cancelled, error.to_string()))?;
    Ok(matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
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
