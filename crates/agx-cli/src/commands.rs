use std::collections::BTreeMap;

use serde::Serialize;

use crate::agents::{self, AgentDefinition};
use crate::cli::Command;
use crate::config;
use crate::context::CliContext;
use crate::doctor;
use crate::errors::{AgxError, AgxErrorCode};
use crate::exec;
use crate::inspection;
use crate::output::{CommandResult, CommandTarget, CommandWarning};
use crate::package_manager;
use crate::self_upgrade;

pub fn run_command(command: &Command, context: &CliContext) -> CommandResult {
    match command {
        Command::Capabilities => capabilities_command(context),
        Command::Commands => commands_command(context),
        Command::Config { action, key, value } => {
            config_command(action.as_deref(), key.as_deref(), value.as_deref(), context)
        }
        Command::Doctor => doctor_command(context),
        Command::Ensure { agent } => ensure_command(agent, context),
        Command::Exec {
            agent,
            args,
            install_policy,
        } => exec_command(agent, args, *install_policy, context),
        Command::External(args) => shortcut_exec_command(args, context),
        Command::Info { agent } => info_command(agent, context),
        Command::Install { agents } => install_command(agents, context),
        Command::Inspect { agent } => inspect_command(agent, context),
        Command::List => list_command(context),
        Command::Resolve { agent } => resolve_command(agent, context),
        Command::Schema { command } => schema_command(command.as_deref(), context),
        Command::Uninstall { agent } => uninstall_command(agent, context),
        Command::Upgrade { channel, check } => upgrade_command(
            channel.map(|channel| match channel {
                crate::cli::SelfUpdateChannelArg::Stable => self_upgrade::SelfUpdateChannel::Stable,
                crate::cli::SelfUpdateChannelArg::Beta => self_upgrade::SelfUpdateChannel::Beta,
            }),
            *check,
            context,
        ),
        Command::Update { agent, all } => update_command(agent.as_deref(), *all, context),
    }
}

fn doctor_command(context: &CliContext) -> CommandResult {
    CommandResult::success(
        "doctor",
        doctor::run_doctor(context),
        CommandTarget::system("doctor"),
        context,
    )
}

fn shortcut_exec_command(args: &[String], context: &CliContext) -> CommandResult {
    if !matches!(context.output_mode, crate::context::OutputMode::Human) {
        return CommandResult::error(
            "exec",
            AgxError::new(
                AgxErrorCode::InvalidArgument,
                "Structured output is not supported for shortcut agent execution yet. Use `agx exec <agent>` instead.",
            ),
            CommandTarget::agent(""),
            context,
        );
    }

    let Some((agent_name, agent_args)) = args.split_first() else {
        return CommandResult::error(
            "exec",
            AgxError::new(
                AgxErrorCode::InvalidArgument,
                "Please specify an agent name",
            ),
            CommandTarget::agent(""),
            context,
        );
    };

    let agent_args = if agent_args.first().is_some_and(|arg| arg == "--") {
        &agent_args[1..]
    } else {
        agent_args
    };

    if let Some(agent) = agents::resolve_agent(agent_name)
        && inspection::find_binary_in_path(agent.binary_name).is_none()
    {
        if !context.interactive && !context.assume_yes {
            let result = exec_missing_result(
                agent,
                agent_args,
                crate::cli::InstallPolicyArg::IfMissing,
                context,
                AgxErrorCode::InteractionRequired,
                format!(
                    "{} is not installed and interactive installation is disabled.",
                    agent.display_name
                ),
            );
            return result;
        }

        if context.interactive && !context.assume_yes && !confirm_shortcut_install(agent, context) {
            return CommandResult::error(
                "exec",
                AgxError::new(
                    AgxErrorCode::Cancelled,
                    format!("Installation cancelled for {}.", agent.display_name),
                ),
                CommandTarget::agent(agent.name),
                context,
            );
        }
    }

    exec_command(
        agent_name,
        agent_args,
        crate::cli::InstallPolicyArg::IfMissing,
        context,
    )
}

fn confirm_shortcut_install(agent: AgentDefinition, context: &CliContext) -> bool {
    if !context.interactive || context.assume_yes {
        return true;
    }

    eprintln!(
        "{} is not installed. Install it now? [y/N]",
        agent.display_name
    );

    if let Ok(answer) = std::env::var("AGX_TEST_PROMPT_RESPONSE") {
        return matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes");
    }

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .is_ok_and(|_| matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes"))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandDescriptor {
    flags: Vec<&'static str>,
    name: &'static str,
    output_schema_ref: &'static str,
    stability: &'static str,
    summary: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandsData {
    commands: Vec<CommandDescriptor>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigData {
    action: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    config: Option<std::collections::BTreeMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CapabilitiesData {
    agents: Vec<&'static str>,
    features: FeatureCapabilities,
    installers: InstallerCapabilities,
    output_modes: Vec<&'static str>,
    platform: PlatformCapabilities,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ListData {
    agents: Vec<ListedAgent>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ListedAgent {
    binary_name: &'static str,
    display_name: &'static str,
    installed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    installed_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latest_version: Option<String>,
    lifecycle: String,
    name: &'static str,
    source_label: String,
    update_label: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InfoData {
    agent: AgentInfo,
    inspection: inspection::AgentInspection,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LifecycleData {
    agent: LifecycleAgent,
    changed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    install_state: Option<crate::state::InstalledAgentState>,
    installed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LifecycleBatchData {
    results: Vec<LifecycleBatchResultItem>,
    scope: &'static str,
    summary: LifecycleBatchSummary,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LifecycleBatchResultItem {
    agent: LifecycleAgent,
    changed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<BatchErrorData>,
    input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    install_state: Option<crate::state::InstalledAgentState>,
    installed: bool,
    ok: bool,
    status: &'static str,
    warnings: Vec<CommandWarning>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BatchErrorData {
    code: AgxErrorCode,
    message: String,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct LifecycleBatchSummary {
    already_installed: usize,
    failed: usize,
    installed: usize,
    locked: usize,
    planned: usize,
    tracked_existing_install: usize,
    untracked_existing_install: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LifecycleAgent {
    display_name: String,
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateData {
    results: Vec<package_manager::UpdateResult>,
    scope: &'static str,
}

#[derive(Debug, Clone)]
struct PendingGroupedUpdate {
    agent: AgentDefinition,
    installed_state: crate::state::InstalledAgentState,
    installed_version: Option<String>,
    latest_version: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentInfo {
    aliases: Vec<&'static str>,
    binary_name: &'static str,
    display_name: &'static str,
    homepage: &'static str,
    install_methods: Vec<InstallMethodInfo>,
    name: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    package_name: Option<&'static str>,
    self_update_commands: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectData {
    agent: AgentInfo,
    capabilities: AgentCapabilities,
    inspection: inspection::AgentInspection,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
struct AgentCapabilities {
    can_auto_install: bool,
    can_auto_uninstall: bool,
    can_run: bool,
    can_self_update: bool,
    install_methods: Vec<InstallMethodInfo>,
    self_update_commands: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolveData {
    agent: AgentInfo,
    resolution: Resolution,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Resolution {
    #[serde(skip_serializing_if = "Option::is_none")]
    binary_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    install_guidance: Option<InstallGuidance>,
    installed: bool,
    install_source: &'static str,
    lifecycle: String,
    source_label: String,
    suggested_launch_command: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallGuidance {
    docs_ref: &'static str,
    install_methods: Vec<InstallMethodInfo>,
    suggested_action: &'static str,
    suggested_ensure_command: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallMethodInfo {
    command: String,
    label: &'static str,
    #[serde(rename = "type")]
    method_type: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExecCommandData {
    agent: exec::ExecAgent,
    execution: exec::ExecExecution,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
struct FeatureCapabilities {
    assume_yes: bool,
    cache_bypass: bool,
    cache_refresh: bool,
    channels: Vec<&'static str>,
    color_modes: Vec<&'static str>,
    dry_run: bool,
    exec_install_policies: Vec<&'static str>,
    freshness_metadata: bool,
    idempotency_key: bool,
    log_levels: Vec<&'static str>,
    quiet_logs: bool,
    self_upgrade: bool,
    timeout: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallerCapabilities {
    brew: InstallerAvailability,
    bun: InstallerAvailability,
    npm: InstallerAvailability,
    winget: InstallerAvailability,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallerAvailability {
    available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlatformCapabilities {
    arch: &'static str,
    os: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SchemaData {
    commands: Vec<SchemaDocument>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SchemaDocument {
    data_schema: JsonSchema,
    description: &'static str,
    envelope_schema: JsonSchema,
    name: &'static str,
    ndjson_event_schema: JsonSchema,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct JsonSchema {
    #[serde(skip_serializing_if = "Option::is_none")]
    additional_properties: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<Box<JsonSchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<Vec<SchemaProperty>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<Vec<&'static str>>,
    #[serde(rename = "type")]
    schema_type: &'static str,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SchemaProperty {
    name: &'static str,
    schema: JsonSchema,
}

fn commands_command(context: &CliContext) -> CommandResult {
    CommandResult::success(
        "commands",
        CommandsData {
            commands: command_catalog(),
        },
        CommandTarget::system("commands"),
        context,
    )
}

fn capabilities_command(context: &CliContext) -> CommandResult {
    CommandResult::success(
        "capabilities",
        CapabilitiesData {
            agents: supported_agents(),
            features: FeatureCapabilities {
                assume_yes: true,
                cache_bypass: true,
                cache_refresh: true,
                channels: vec!["stable", "beta"],
                color_modes: vec!["auto", "always", "never"],
                dry_run: true,
                exec_install_policies: vec!["never", "if-missing", "always"],
                freshness_metadata: true,
                idempotency_key: true,
                log_levels: vec!["silent", "error", "warn", "info", "debug"],
                quiet_logs: true,
                self_upgrade: true,
                timeout: true,
            },
            installers: InstallerCapabilities {
                brew: installer_availability("brew"),
                bun: installer_availability("bun"),
                npm: installer_availability("npm"),
                winget: installer_availability("winget"),
            },
            output_modes: vec!["human", "json", "ndjson"],
            platform: PlatformCapabilities {
                arch: std::env::consts::ARCH,
                os: std::env::consts::OS,
            },
        },
        CommandTarget::system("capabilities"),
        context,
    )
}

fn config_command(
    action: Option<&str>,
    key: Option<&str>,
    value: Option<&str>,
    context: &CliContext,
) -> CommandResult {
    match action {
        None => CommandResult::success(
            "config",
            ConfigData {
                action: "list",
                config: Some(config::load_effective_config()),
                key: None,
                value: None,
            },
            CommandTarget::config(None),
            context,
        ),
        Some("get") => {
            let Some(key) = key else {
                return invalid_config_argument("Please specify a key", None, context);
            };
            CommandResult::success(
                "config",
                ConfigData {
                    action: "get",
                    config: None,
                    key: Some(key.to_string()),
                    value: Some(config::get_config_value(key)),
                },
                CommandTarget::config(Some(key.to_string())),
                context,
            )
        }
        Some("set") => {
            let (Some(key), Some(value)) = (key, value) else {
                return invalid_config_argument("Please specify both key and value", None, context);
            };
            match config::set_config_value(key, value) {
                Ok(stored) => CommandResult::success(
                    "config",
                    ConfigData {
                        action: "set",
                        config: None,
                        key: Some(key.to_string()),
                        value: Some(stored),
                    },
                    CommandTarget::config(Some(key.to_string())),
                    context,
                ),
                Err(error) => CommandResult::error(
                    "config",
                    error,
                    CommandTarget::config(Some(key.to_string())),
                    context,
                ),
            }
        }
        Some("reset") => match config::reset_config() {
            Ok(defaults) => CommandResult::success(
                "config",
                ConfigData {
                    action: "reset",
                    config: Some(defaults),
                    key: None,
                    value: None,
                },
                CommandTarget::config(None),
                context,
            ),
            Err(error) => {
                CommandResult::error("config", error, CommandTarget::config(None), context)
            }
        },
        Some(other) => invalid_config_argument(format!("Unknown action: {other}"), None, context),
    }
}

fn list_command(context: &CliContext) -> CommandResult {
    CommandResult::success(
        "list",
        ListData {
            agents: agents::all_agents()
                .iter()
                .map(|agent| {
                    let inspection = resolved_agent_inspection(*agent, context);
                    ListedAgent {
                        binary_name: agent.binary_name,
                        display_name: agent.display_name,
                        installed: inspection.installed,
                        installed_version: inspection.installed_version,
                        latest_version: inspection.latest_version,
                        lifecycle: inspection.lifecycle,
                        name: agent.name,
                        source_label: inspection.source_label,
                        update_label: inspection.update_label,
                    }
                })
                .collect(),
        },
        CommandTarget::system("agents"),
        context,
    )
}

fn install_command(agent_names: &[String], context: &CliContext) -> CommandResult {
    if agent_names.len() <= 1 {
        return lifecycle_command(
            "install",
            agent_names
                .first()
                .expect("clap should require at least one install target"),
            context,
            package_manager::install_agent,
        );
    }

    let _ = crate::output::emit_ndjson_event(
        "install",
        "started",
        serde_json::json!({ "scope": "batch" }),
        Some(CommandTarget {
            kind: crate::output::TargetKind::Agent,
            name: None,
        }),
        context,
    );

    let mut results = Vec::new();
    match crate::lock::acquire_resource_lock("agent lifecycle") {
        Ok(_lock_guard) => {
            for agent_name in agent_names {
                let result = lifecycle_command_with_started(
                    "install",
                    agent_name,
                    context,
                    package_manager::install_agent,
                    false,
                );
                let batch_result = lifecycle_batch_result_item(agent_name, &result);
                let target_name = batch_result.agent.name.clone();

                let _ = crate::output::emit_ndjson_event(
                    "install",
                    "progress",
                    &batch_result,
                    Some(CommandTarget::agent(target_name)),
                    context,
                );

                results.push(batch_result);
            }
        }
        Err(error) => {
            let result = CommandResult::error(
                "install",
                error,
                CommandTarget {
                    kind: crate::output::TargetKind::Agent,
                    name: None,
                },
                context,
            );
            results.push(lifecycle_batch_result_item("batch", &result));
        }
    }

    let summary = summarize_lifecycle_batch_results(&results);
    let data = LifecycleBatchData {
        results,
        scope: "batch",
        summary,
    };

    if data.results.iter().any(|result| !result.ok) {
        let error = batch_lifecycle_error("install", &data.results);
        return CommandResult::error_with_data(
            "install",
            data,
            error,
            CommandTarget {
                kind: crate::output::TargetKind::Agent,
                name: None,
            },
            context,
        );
    }

    CommandResult::success(
        "install",
        data,
        CommandTarget {
            kind: crate::output::TargetKind::Agent,
            name: None,
        },
        context,
    )
}

fn ensure_command(agent_name: &str, context: &CliContext) -> CommandResult {
    lifecycle_command("ensure", agent_name, context, package_manager::ensure_agent)
}

fn exec_command(
    agent_name: &str,
    args: &[String],
    install_policy: crate::cli::InstallPolicyArg,
    context: &CliContext,
) -> CommandResult {
    let Some(agent) = agents::resolve_agent(agent_name) else {
        return agent_not_found_result("exec", agent_name, context);
    };

    match exec::execute_agent(agent, args, install_policy, context) {
        Ok(result) => {
            let exit_code = result.execution.exit_code.unwrap_or(0);
            CommandResult::success_with_exit_code(
                "exec",
                result,
                CommandTarget::agent(agent.name),
                context,
                exit_code,
            )
        }
        Err(error)
            if matches!(
                error.code,
                AgxErrorCode::AgentNotInstalled | AgxErrorCode::InteractionRequired
            ) =>
        {
            exec_missing_result(
                agent,
                args,
                install_policy,
                context,
                error.code,
                error.message,
            )
        }
        Err(error) => {
            CommandResult::error("exec", error, CommandTarget::agent(agent.name), context)
        }
    }
}

fn uninstall_command(agent_name: &str, context: &CliContext) -> CommandResult {
    lifecycle_command(
        "uninstall",
        agent_name,
        context,
        package_manager::uninstall_agent,
    )
}

fn upgrade_command(
    channel: Option<self_upgrade::SelfUpdateChannel>,
    check: bool,
    context: &CliContext,
) -> CommandResult {
    let inspection = self_upgrade::inspect_self_with_context(channel, context);
    let target = CommandTarget {
        kind: crate::output::TargetKind::SelfTarget,
        name: Some("agx".to_string()),
    };
    match self_upgrade::upgrade_self(context, channel, check) {
        Ok(result) => {
            let stale_latest = result
                .latest_version
                .as_deref()
                .zip(result.current_version.as_deref())
                .is_some_and(|(latest, current)| self_upgrade::is_version_older(latest, current));
            let mirror_lag = inspection
                .latest_version
                .as_deref()
                .zip(inspection.upstream_latest_version.as_deref())
                .is_some_and(|(latest, upstream)| latest != upstream)
                && matches!(
                    inspection.install_source,
                    self_upgrade::InstallSourceKind::Bun | self_upgrade::InstallSourceKind::Npm
                );
            let mut command_result = if check && result.status == "update-available" {
                CommandResult::success_with_exit_code("upgrade", result, target, context, 1)
            } else {
                CommandResult::success("upgrade", result, target, context)
            };
            if stale_latest {
                command_result.warnings.push(CommandWarning {
                    code: "STALE_LATEST_VERSION".to_string(),
                    message: "Selected registry reported a version older than the current AGX build; downgrade was skipped.".to_string(),
                });
            }
            if mirror_lag {
                command_result.warnings.push(CommandWarning {
                    code: "MIRROR_LAG".to_string(),
                    message: format!(
                        "The selected registry currently installs {}, while upstream npm has {}. Retry later or set selfUpdateRegistry to another registry if you need the upstream release now.",
                        inspection.latest_version.as_deref().unwrap_or("unknown"),
                        inspection.upstream_latest_version.as_deref().unwrap_or("unknown")
                    ),
                });
            }
            if context.dry_run {
                command_result.warnings.push(dry_run_warning());
            }
            command_result
        }
        Err(error) if check && matches!(error.code, AgxErrorCode::NetworkError) => {
            CommandResult::error_with_data(
                "upgrade",
                self_upgrade::UpgradeData {
                    channel: Some(inspection.update_channel),
                    command: Vec::new(),
                    current_version: Some(inspection.current_version),
                    dry_run: context.dry_run,
                    install_source: inspection.install_source,
                    latest_version: inspection.latest_version,
                    recovery_hint: None,
                    message: Some(error.message.clone()),
                    package_name: "agxctl",
                    status: "check-unavailable",
                    verified_version: None,
                },
                error,
                target,
                context,
            )
        }
        Err(error) => CommandResult::error_with_data(
            "upgrade",
            self_upgrade::UpgradeData {
                channel: Some(inspection.update_channel),
                command: Vec::new(),
                current_version: Some(inspection.current_version),
                dry_run: context.dry_run,
                install_source: inspection.install_source,
                latest_version: inspection.latest_version,
                recovery_hint: self_upgrade::get_recovery_hint(
                    inspection.install_source,
                    inspection.update_channel,
                ),
                message: Some(error.message.clone()),
                package_name: "agxctl",
                status: if matches!(error.code, AgxErrorCode::ManualActionRequired) {
                    "manual-required"
                } else {
                    "failed"
                },
                verified_version: None,
            },
            error,
            target,
            context,
        ),
    }
}

#[allow(clippy::too_many_lines)]
fn update_command(agent_name: Option<&str>, all: bool, context: &CliContext) -> CommandResult {
    if all {
        let _ = crate::output::emit_ndjson_event(
            "update",
            "started",
            serde_json::json!({ "scope": "all" }),
            Some(CommandTarget::agent("all")),
            context,
        );
        let mut results = Vec::new();
        match crate::lock::acquire_resource_lock("agent lifecycle") {
            Ok(_lock_guard) => {
                let mut grouped_updates: BTreeMap<String, Vec<PendingGroupedUpdate>> =
                    BTreeMap::new();
                for agent in agents::all_agents() {
                    let inspection = resolved_agent_inspection(*agent, context);
                    let installed_state = crate::state::get_installed_agent_state(agent.name);

                    if !inspection.installed && installed_state.is_none() {
                        continue;
                    }

                    if inspection.installed && installed_state.is_none() {
                        let result = package_manager::UpdateResult {
                            display_name: agent.display_name.to_string(),
                            hint: Some(format!(
                                "Use `agx inspect {} --json` to confirm the source, then reinstall through AGX if you want `agx update --all` to manage it.",
                                agent.name
                            )),
                            installed_version: inspection.installed_version,
                            latest_version: inspection.latest_version,
                            name: agent.name.to_string(),
                            message: Some(format!(
                                "{} is detected in PATH but not tracked as an AGX-managed install.",
                                agent.display_name
                            )),
                            resource: None,
                            status: "manual-required",
                            strategy: Some("manual-hint".to_string()),
                        };
                        push_update_result(&mut results, result, context);
                        continue;
                    }

                    if inspection.installed_version.is_some()
                        && inspection.installed_version == inspection.latest_version
                    {
                        let result = package_manager::UpdateResult {
                            display_name: agent.display_name.to_string(),
                            hint: None,
                            installed_version: inspection.installed_version,
                            latest_version: inspection.latest_version,
                            name: agent.name.to_string(),
                            message: None,
                            resource: None,
                            status: "up-to-date",
                            strategy: Some(inspection.update_label.clone()),
                        };
                        push_update_result(&mut results, result, context);
                        continue;
                    }

                    if inspection.latest_version.is_none()
                        && agent.npm_package.is_none()
                        && agents::self_update_commands(*agent).is_empty()
                    {
                        let result = package_manager::UpdateResult {
                            display_name: agent.display_name.to_string(),
                            hint: Some(format!(
                                "Check {} for the recommended update path.",
                                agent.homepage
                            )),
                            installed_version: inspection.installed_version,
                            latest_version: inspection.latest_version,
                            name: agent.name.to_string(),
                            message: Some(format!(
                                "{} uses a manually managed install source. Please check for updates manually.",
                                agent.display_name
                            )),
                            resource: None,
                            status: "manual-required",
                            strategy: Some("manual-hint".to_string()),
                        };
                        push_update_result(&mut results, result, context);
                        continue;
                    }

                    if let Some(installed_state) = installed_state.as_ref()
                        && matches!(installed_state.install_type.as_str(), "bun" | "npm")
                        && installed_state.package_name.is_some()
                    {
                        grouped_updates
                            .entry(installed_state.install_type.clone())
                            .or_default()
                            .push(PendingGroupedUpdate {
                                agent: *agent,
                                installed_state: installed_state.clone(),
                                installed_version: inspection.installed_version.clone(),
                                latest_version: inspection.latest_version.clone(),
                            });
                        continue;
                    }

                    let result = perform_update(
                        *agent,
                        installed_state.as_ref(),
                        inspection.installed_version.as_deref(),
                        inspection.latest_version.as_deref(),
                        context,
                        Some(inspection.update_label.clone()),
                    );
                    push_update_result(&mut results, result, context);
                }

                for bucket in grouped_updates.into_values() {
                    for result in perform_grouped_updates(bucket, context) {
                        push_update_result(&mut results, result, context);
                    }
                }

                for installed_state in crate::state::load_state().installed_agents.into_values() {
                    if agents::resolve_agent(&installed_state.agent_name).is_none() {
                        let result = package_manager::UpdateResult {
                            display_name: installed_state.agent_name.clone(),
                            hint: None,
                            installed_version: None,
                            latest_version: None,
                            name: installed_state.agent_name.clone(),
                            message: Some(
                                "Tracked agent is no longer in the AGX catalog.".to_string(),
                            ),
                            resource: None,
                            status: "manual-required",
                            strategy: None,
                        };
                        push_update_result(&mut results, result, context);
                    }
                }
            }
            Err(error) => {
                push_update_result(
                    &mut results,
                    package_manager::UpdateResult {
                        display_name: "AGX agent lifecycle".to_string(),
                        hint: None,
                        installed_version: None,
                        latest_version: None,
                        name: "all".to_string(),
                        message: Some(error.message),
                        resource: None,
                        status: "locked",
                        strategy: None,
                    },
                    context,
                );
            }
        }

        let has_failures = results
            .iter()
            .any(|result| matches!(result.status, "failed" | "locked"));
        let has_only_locks = has_failures
            && results
                .iter()
                .filter(|result| matches!(result.status, "failed" | "locked"))
                .all(|result| result.status == "locked");
        let data = UpdateData {
            results,
            scope: "all",
        };
        if has_failures {
            return CommandResult::error_with_data(
                "update",
                data,
                AgxError::new(
                    if has_only_locks {
                        AgxErrorCode::ResourceLocked
                    } else {
                        AgxErrorCode::UpdateFailed
                    },
                    if has_only_locks {
                        "Another AGX process already holds the agent lifecycle lock."
                    } else {
                        "One or more agents failed to update."
                    },
                ),
                CommandTarget::agent("all"),
                context,
            );
        }
        return CommandResult::success("update", data, CommandTarget::agent("all"), context);
    }

    let Some(agent_name) = agent_name else {
        return CommandResult::error(
            "update",
            AgxError::new(
                AgxErrorCode::InvalidArgument,
                "Please specify an agent name or use --all flag",
            ),
            CommandTarget::agent(""),
            context,
        );
    };

    let Some(agent) = agents::resolve_agent(agent_name) else {
        return agent_not_found_result("update", agent_name, context);
    };

    let _ = crate::output::emit_ndjson_event(
        "update",
        "started",
        serde_json::json!({ "scope": "single", "agent": agent.name }),
        Some(CommandTarget::agent(agent.name)),
        context,
    );

    let inspection = resolved_agent_inspection(agent, context);
    let installed_state = crate::state::get_installed_agent_state(agent.name);
    if !inspection.installed && installed_state.is_none() {
        return CommandResult::error(
            "update",
            AgxError::new(
                AgxErrorCode::AgentNotInstalled,
                format!("{} is not installed.", agent.display_name),
            ),
            CommandTarget::agent(agent.name),
            context,
        );
    }

    if inspection.installed_version.is_some()
        && inspection.installed_version == inspection.latest_version
    {
        let result = package_manager::UpdateResult {
            display_name: agent.display_name.to_string(),
            hint: None,
            installed_version: inspection.installed_version,
            latest_version: inspection.latest_version,
            name: agent.name.to_string(),
            message: None,
            resource: None,
            status: "up-to-date",
            strategy: Some(inspection.update_label.clone()),
        };
        emit_update_progress(&result, context);
        return CommandResult::success(
            "update",
            UpdateData {
                results: vec![result],
                scope: "single",
            },
            CommandTarget::agent(agent.name),
            context,
        );
    }

    match crate::lock::with_resource_lock("agent lifecycle", || {
        package_manager::update_agent(
            agent,
            installed_state.as_ref(),
            inspection.installed_version.clone(),
            inspection.latest_version.clone(),
            context,
        )
    }) {
        Ok(result) if matches!(result.status, "failed" | "locked") => {
            CommandResult::error_with_data(
                "update",
                UpdateData {
                    results: vec![result.clone()],
                    scope: "single",
                },
                AgxError::new(
                    if result.status == "locked" {
                        AgxErrorCode::ResourceLocked
                    } else {
                        AgxErrorCode::UpdateFailed
                    },
                    result
                        .message
                        .clone()
                        .unwrap_or_else(|| format!("Failed to update {}.", agent.display_name)),
                ),
                CommandTarget::agent(agent.name),
                context,
            )
        }
        Err(error) => CommandResult::error_with_data(
            "update",
            UpdateData {
                results: vec![package_manager::UpdateResult {
                    display_name: agent.display_name.to_string(),
                    hint: None,
                    installed_version: inspection.installed_version.clone(),
                    latest_version: inspection.latest_version.clone(),
                    name: agent.name.to_string(),
                    message: Some(error.message.clone()),
                    resource: None,
                    status: if matches!(error.code, AgxErrorCode::ResourceLocked) {
                        "locked"
                    } else {
                        "failed"
                    },
                    strategy: Some(inspection.update_label.clone()),
                }],
                scope: "single",
            },
            error,
            CommandTarget::agent(agent.name),
            context,
        ),
        Ok(result) => CommandResult::success(
            "update",
            UpdateData {
                results: vec![{
                    let normalized = normalize_update_result(
                        agent,
                        result,
                        inspection.installed_version.as_deref(),
                    );
                    emit_update_progress(&normalized, context);
                    normalized
                }],
                scope: "single",
            },
            CommandTarget::agent(agent.name),
            context,
        ),
    }
}

fn emit_update_progress(result: &package_manager::UpdateResult, context: &CliContext) {
    let _ = crate::output::emit_ndjson_event(
        "update",
        "progress",
        result,
        Some(CommandTarget::agent(result.name.clone())),
        context,
    );
}

fn push_update_result(
    results: &mut Vec<package_manager::UpdateResult>,
    result: package_manager::UpdateResult,
    context: &CliContext,
) {
    emit_update_progress(&result, context);
    results.push(result);
}

fn perform_grouped_updates(
    bucket: Vec<PendingGroupedUpdate>,
    context: &CliContext,
) -> Vec<package_manager::UpdateResult> {
    if bucket.is_empty() {
        return Vec::new();
    }

    let install_type = bucket[0].installed_state.install_type.clone();
    if context.dry_run {
        return bucket
            .into_iter()
            .map(|entry| package_manager::UpdateResult {
                display_name: entry.agent.display_name.to_string(),
                hint: None,
                installed_version: entry.installed_version,
                latest_version: entry.latest_version,
                name: entry.agent.name.to_string(),
                message: Some(format!(
                    "Dry run: would update {}.",
                    entry.agent.display_name
                )),
                resource: None,
                status: "planned",
                strategy: Some(format!("managed/{install_type}")),
            })
            .collect();
    }

    let packages = bucket
        .iter()
        .filter_map(|entry| entry.installed_state.package_name.clone())
        .collect::<Vec<_>>();

    match package_manager::update_agents_by_type(&install_type, &packages) {
        Ok(()) => bucket
            .into_iter()
            .map(|entry| package_manager::UpdateResult {
                display_name: entry.agent.display_name.to_string(),
                hint: None,
                installed_version: entry.installed_version,
                latest_version: entry.latest_version,
                name: entry.agent.name.to_string(),
                message: None,
                resource: None,
                status: "updated",
                strategy: Some(format!("managed/{install_type}")),
            })
            .collect(),
        Err(error) if matches!(error.code, AgxErrorCode::ResourceLocked) => bucket
            .into_iter()
            .map(|entry| package_manager::UpdateResult {
                display_name: entry.agent.display_name.to_string(),
                hint: None,
                installed_version: entry.installed_version,
                latest_version: entry.latest_version,
                name: entry.agent.name.to_string(),
                message: Some(error.message.clone()),
                resource: None,
                status: "locked",
                strategy: Some(format!("managed/{install_type}")),
            })
            .collect(),
        Err(_) => bucket
            .into_iter()
            .map(|entry| {
                perform_update(
                    entry.agent,
                    Some(&entry.installed_state),
                    entry.installed_version.as_deref(),
                    entry.latest_version.as_deref(),
                    context,
                    Some(format!("managed/{install_type}")),
                )
            })
            .collect(),
    }
}

fn perform_update(
    agent: AgentDefinition,
    installed_state: Option<&crate::state::InstalledAgentState>,
    installed_version: Option<&str>,
    latest_version: Option<&str>,
    context: &CliContext,
    fallback_strategy: Option<String>,
) -> package_manager::UpdateResult {
    let installed_version_owned = installed_version.map(str::to_string);
    let latest_version_owned = latest_version.map(str::to_string);
    normalize_update_result(
        agent,
        package_manager::update_agent(
            agent,
            installed_state,
            installed_version_owned.clone(),
            latest_version_owned.clone(),
            context,
        )
        .unwrap_or_else(|error| package_manager::UpdateResult {
            display_name: agent.display_name.to_string(),
            hint: None,
            installed_version: installed_version_owned.clone(),
            latest_version: latest_version_owned.clone(),
            name: agent.name.to_string(),
            message: Some(error.message),
            resource: None,
            status: if matches!(error.code, AgxErrorCode::ResourceLocked) {
                "locked"
            } else {
                "failed"
            },
            strategy: fallback_strategy,
        }),
        installed_version,
    )
}

fn normalize_update_result(
    agent: AgentDefinition,
    mut result: package_manager::UpdateResult,
    installed_version_before: Option<&str>,
) -> package_manager::UpdateResult {
    if result.status != "updated" || result.strategy.as_deref() != Some("self-update") {
        return result;
    }

    let Some(previous_version) = installed_version_before else {
        return result;
    };
    let Some(binary_path) = inspection::find_binary_in_path(agent.binary_name) else {
        return result;
    };
    let Some(observed_version) = inspection::probe_binary_version(&binary_path) else {
        return result;
    };

    if observed_version == previous_version {
        result.installed_version = Some(observed_version.clone());
        result.latest_version = Some(observed_version);
        result.status = "up-to-date";
    } else {
        result.latest_version = Some(observed_version);
    }

    result
}

fn info_command(agent_name: &str, context: &CliContext) -> CommandResult {
    let Some(agent) = agents::resolve_agent(agent_name) else {
        return agent_not_found_result("info", agent_name, context);
    };

    CommandResult::success(
        "info",
        InfoData {
            agent: agent_info(agent),
            inspection: resolved_agent_inspection(agent, context),
        },
        CommandTarget::agent(agent.name),
        context,
    )
}

fn inspect_command(agent_name: &str, context: &CliContext) -> CommandResult {
    let Some(agent) = agents::resolve_agent(agent_name) else {
        return agent_not_found_result("inspect", agent_name, context);
    };

    CommandResult::success(
        "inspect",
        InspectData {
            agent: agent_info(agent),
            capabilities: agent_capabilities(agent),
            inspection: resolved_agent_inspection(agent, context),
        },
        CommandTarget::agent(agent.name),
        context,
    )
}

fn resolve_command(agent_name: &str, context: &CliContext) -> CommandResult {
    let Some(agent) = agents::resolve_agent(agent_name) else {
        return agent_not_found_result("resolve", agent_name, context);
    };

    let inspection = resolved_agent_inspection(agent, context);
    let install_methods = install_methods(agent);
    let installed = inspection.installed;
    let suggested_launch_command = inspection.binary_path.as_ref().map_or_else(
        || vec![agent.binary_name.to_string()],
        |binary_path| vec![binary_path.clone()],
    );
    let resolution = Resolution {
        binary_path: inspection.binary_path.clone(),
        install_guidance: (!installed).then_some(InstallGuidance {
            docs_ref: "openspec/changes/rewrite-quantex-cli-as-agx-rust/tasks.md",
            install_methods,
            suggested_action: "ensure-agent-installed",
            suggested_ensure_command: format!("agx ensure {}", agent.name),
        }),
        installed,
        install_source: install_source_for(agent),
        lifecycle: inspection.lifecycle,
        source_label: inspection.source_label,
        suggested_launch_command,
    };
    let data = ResolveData {
        agent: agent_info(agent),
        resolution,
    };

    if installed {
        CommandResult::success("resolve", data, CommandTarget::agent(agent.name), context)
    } else {
        CommandResult::error_with_data(
            "resolve",
            data,
            AgxError::new(
                AgxErrorCode::AgentNotInstalled,
                format!(
                    "{} is not installed. Run `agx ensure {}` first.",
                    agent.display_name, agent.name
                ),
            ),
            CommandTarget::agent(agent.name),
            context,
        )
    }
}

fn schema_command(command_name: Option<&str>, context: &CliContext) -> CommandResult {
    let mut commands = schema_catalog();
    if let Some(command_name) = command_name {
        commands.retain(|schema| schema.name == command_name);
        if commands.is_empty() {
            return CommandResult::error(
                "schema",
                AgxError::new(
                    AgxErrorCode::InvalidArgument,
                    format!("Unknown schema target: {command_name}"),
                ),
                CommandTarget::system("schema"),
                context,
            );
        }
    }

    CommandResult::success(
        "schema",
        SchemaData { commands },
        CommandTarget::system("schema"),
        context,
    )
}

fn lifecycle_command(
    action: &'static str,
    agent_name: &str,
    context: &CliContext,
    operation: fn(
        AgentDefinition,
        &CliContext,
    ) -> Result<package_manager::LifecycleResult, AgxError>,
) -> CommandResult {
    lifecycle_command_with_started(action, agent_name, context, operation, true)
}

fn lifecycle_command_with_started(
    action: &'static str,
    agent_name: &str,
    context: &CliContext,
    operation: fn(
        AgentDefinition,
        &CliContext,
    ) -> Result<package_manager::LifecycleResult, AgxError>,
    emit_started: bool,
) -> CommandResult {
    let Some(agent) = agents::resolve_agent(agent_name) else {
        return agent_not_found_result(action, agent_name, context);
    };

    if emit_started {
        let _ = crate::output::emit_ndjson_event(
            action,
            "started",
            serde_json::json!({ "agent": agent.name }),
            Some(CommandTarget::agent(agent.name)),
            context,
        );
    }

    let result = if emit_started {
        crate::lock::with_resource_lock("agent lifecycle", || operation(agent, context))
    } else {
        operation(agent, context)
    };

    match result {
        Ok(result) => {
            let mut command_result = CommandResult::success(
                action,
                LifecycleData {
                    agent: LifecycleAgent {
                        display_name: agent.display_name.to_string(),
                        name: agent.name.to_string(),
                    },
                    changed: result.changed,
                    install_state: result.install_state,
                    installed: result.installed,
                    message: result.message,
                },
                CommandTarget::agent(agent.name),
                context,
            );
            if context.dry_run {
                command_result.warnings.push(dry_run_warning());
            }
            command_result
        }
        Err(error) => {
            CommandResult::error(action, error, CommandTarget::agent(agent.name), context)
        }
    }
}

fn lifecycle_batch_result_item(input: &str, result: &CommandResult) -> LifecycleBatchResultItem {
    let data = result.data.as_ref();
    let agent = data
        .map(|value| LifecycleAgent {
            display_name: value["agent"]["displayName"]
                .as_str()
                .unwrap_or(input)
                .to_string(),
            name: value["agent"]["name"]
                .as_str()
                .or_else(|| {
                    result
                        .target
                        .as_ref()
                        .and_then(|target| target.name.as_deref())
                })
                .unwrap_or(input)
                .to_string(),
        })
        .or_else(|| {
            result.target.as_ref().map(|target| LifecycleAgent {
                display_name: input.to_string(),
                name: target.name.clone().unwrap_or_else(|| input.to_string()),
            })
        })
        .unwrap_or(LifecycleAgent {
            display_name: input.to_string(),
            name: input.to_string(),
        });

    let warnings = result.warnings.clone();
    LifecycleBatchResultItem {
        agent,
        changed: data
            .and_then(|value| value.get("changed"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        error: result.error.as_ref().map(|error| BatchErrorData {
            code: error.code,
            message: error.message.clone(),
        }),
        input: input.to_string(),
        install_state: data
            .and_then(|value| value.get("installState"))
            .and_then(|value| serde_json::from_value(value.clone()).ok()),
        installed: data
            .and_then(|value| value.get("installed"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        ok: result.ok,
        status: lifecycle_batch_status(result),
        warnings,
    }
}

fn lifecycle_batch_status(result: &CommandResult) -> &'static str {
    if !result.ok {
        return if result
            .error
            .as_ref()
            .is_some_and(|error| error.code == AgxErrorCode::ResourceLocked)
        {
            "locked"
        } else {
            "failed"
        };
    }

    if result
        .warnings
        .iter()
        .any(|warning| warning.code == "DRY_RUN")
    {
        return "planned";
    }

    let message = result
        .data
        .as_ref()
        .and_then(|data| data.get("message"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if message.contains("now tracking the existing install") {
        return "tracked-existing-install";
    }
    if message.contains("already installed, but this install is not tracked") {
        return "untracked-existing-install";
    }
    if message.contains("already installed.") {
        return "already-installed";
    }

    "installed"
}

fn summarize_lifecycle_batch_results(
    results: &[LifecycleBatchResultItem],
) -> LifecycleBatchSummary {
    let mut summary = LifecycleBatchSummary::default();
    for result in results {
        match result.status {
            "already-installed" => summary.already_installed += 1,
            "failed" => summary.failed += 1,
            "installed" => summary.installed += 1,
            "locked" => summary.locked += 1,
            "planned" => summary.planned += 1,
            "tracked-existing-install" => summary.tracked_existing_install += 1,
            "untracked-existing-install" => summary.untracked_existing_install += 1,
            _ => {}
        }
    }
    summary
}

fn batch_lifecycle_error(action: &'static str, results: &[LifecycleBatchResultItem]) -> AgxError {
    let only_locks = results.iter().filter(|result| !result.ok).all(|result| {
        result
            .error
            .as_ref()
            .is_some_and(|error| error.code == AgxErrorCode::ResourceLocked)
    });

    if only_locks {
        AgxError::new(
            AgxErrorCode::ResourceLocked,
            format!(
                "One or more agents could not be processed because another AGX process already holds the lifecycle lock during {action}."
            ),
        )
    } else {
        AgxError::new(
            AgxErrorCode::InstallFailed,
            "One or more agents failed to install.",
        )
    }
}

#[allow(clippy::too_many_lines)]
fn command_catalog() -> Vec<CommandDescriptor> {
    vec![
        CommandDescriptor {
            flags: vec![
                "--json",
                "--output",
                "--non-interactive",
                "--quiet",
                "--color",
                "--log-level",
                "--refresh",
                "--no-cache",
                "--timeout",
            ],
            name: "capabilities",
            output_schema_ref: "#/commands/capabilities",
            stability: "stable",
            summary: "Return environment and surface capabilities",
        },
        CommandDescriptor {
            flags: vec![
                "--json",
                "--output",
                "--quiet",
                "--color",
                "--log-level",
                "--timeout",
            ],
            name: "commands",
            output_schema_ref: "#/commands/commands",
            stability: "stable",
            summary: "Return the stable command catalog",
        },
        CommandDescriptor {
            flags: vec![
                "get",
                "set",
                "reset",
                "--json",
                "--output",
                "--quiet",
                "--color",
                "--log-level",
                "--timeout",
            ],
            name: "config",
            output_schema_ref: "#/commands/config",
            stability: "stable",
            summary: "Read and modify AGX configuration",
        },
        CommandDescriptor {
            flags: vec![
                "--json",
                "--output",
                "--quiet",
                "--color",
                "--log-level",
                "--timeout",
            ],
            name: "doctor",
            output_schema_ref: "#/commands/doctor",
            stability: "stable",
            summary: "Diagnose AGX installation and runtime health",
        },
        CommandDescriptor {
            flags: vec![
                "--channel",
                "--check",
                "--json",
                "--output",
                "--yes",
                "--quiet",
                "--color",
                "--log-level",
                "--dry-run",
                "--timeout",
                "--idempotency-key",
            ],
            name: "ensure",
            output_schema_ref: "#/commands/ensure",
            stability: "stable",
            summary: "Ensure an agent is installed",
        },
        CommandDescriptor {
            flags: vec![
                "--install",
                "--install-policy",
                "--json",
                "--output",
                "--yes",
                "--quiet",
                "--color",
                "--log-level",
                "--dry-run",
                "--timeout",
            ],
            name: "exec",
            output_schema_ref: "#/commands/exec",
            stability: "stable",
            summary: "Execute an agent command",
        },
        CommandDescriptor {
            flags: vec![
                "--json",
                "--output",
                "--quiet",
                "--color",
                "--log-level",
                "--refresh",
                "--no-cache",
                "--timeout",
            ],
            name: "info",
            output_schema_ref: "#/commands/info",
            stability: "stable",
            summary: "Show agent details",
        },
        CommandDescriptor {
            flags: vec![
                "--json",
                "--output",
                "--quiet",
                "--color",
                "--log-level",
                "--refresh",
                "--no-cache",
                "--timeout",
            ],
            name: "inspect",
            output_schema_ref: "#/commands/inspect",
            stability: "stable",
            summary: "Return structured agent state",
        },
        CommandDescriptor {
            flags: vec![
                "--channel",
                "--check",
                "--json",
                "--output",
                "--yes",
                "--quiet",
                "--color",
                "--log-level",
                "--dry-run",
                "--timeout",
                "--idempotency-key",
            ],
            name: "install",
            output_schema_ref: "#/commands/install",
            stability: "stable",
            summary: "Install one or more agents",
        },
        CommandDescriptor {
            flags: vec![
                "--json",
                "--output",
                "--quiet",
                "--color",
                "--log-level",
                "--refresh",
                "--no-cache",
                "--timeout",
            ],
            name: "list",
            output_schema_ref: "#/commands/list",
            stability: "stable",
            summary: "List supported agents",
        },
        CommandDescriptor {
            flags: vec![
                "--json",
                "--output",
                "--quiet",
                "--color",
                "--log-level",
                "--refresh",
                "--no-cache",
                "--timeout",
            ],
            name: "resolve",
            output_schema_ref: "#/commands/resolve",
            stability: "stable",
            summary: "Resolve an agent executable entrypoint",
        },
        CommandDescriptor {
            flags: vec![
                "--json",
                "--output",
                "--quiet",
                "--color",
                "--log-level",
                "--timeout",
            ],
            name: "schema",
            output_schema_ref: "#/commands/schema",
            stability: "stable",
            summary: "Return structured output schemas",
        },
        CommandDescriptor {
            flags: vec![
                "--json",
                "--output",
                "--quiet",
                "--color",
                "--log-level",
                "--dry-run",
                "--timeout",
                "--idempotency-key",
            ],
            name: "uninstall",
            output_schema_ref: "#/commands/uninstall",
            stability: "stable",
            summary: "Uninstall an agent",
        },
        CommandDescriptor {
            flags: vec![
                "--channel",
                "--check",
                "--json",
                "--output",
                "--yes",
                "--quiet",
                "--color",
                "--log-level",
                "--dry-run",
                "--timeout",
            ],
            name: "upgrade",
            output_schema_ref: "#/commands/upgrade",
            stability: "stable",
            summary: "Upgrade AGX through its detected install channel",
        },
        CommandDescriptor {
            flags: vec![
                "--all",
                "--json",
                "--output",
                "--quiet",
                "--color",
                "--log-level",
                "--dry-run",
                "--refresh",
                "--no-cache",
                "--timeout",
                "--idempotency-key",
            ],
            name: "update",
            output_schema_ref: "#/commands/update",
            stability: "stable",
            summary: "Update one or all agents",
        },
    ]
}

fn supported_agents() -> Vec<&'static str> {
    agents::all_agents()
        .iter()
        .map(|agent| agent.name)
        .collect()
}

fn installer_availability(command: &'static str) -> InstallerAvailability {
    let available = is_command_available(command);
    InstallerAvailability {
        available,
        reason: if available {
            None
        } else if (command == "winget" && !cfg!(windows)) || (command == "brew" && cfg!(windows)) {
            Some("not-on-platform")
        } else {
            Some("not-found")
        },
    }
}

fn is_command_available(command: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };

    std::env::split_paths(&path).any(|directory| {
        let candidate = directory.join(command);
        if candidate.is_file() {
            return true;
        }

        cfg!(windows) && directory.join(format!("{command}.exe")).is_file()
    })
}

#[allow(clippy::too_many_lines)]
fn schema_catalog() -> Vec<SchemaDocument> {
    let envelope_schema = object_schema(vec![
        ("action", string_schema()),
        ("data", object_schema(Vec::new())),
        (
            "error",
            object_schema(vec![
                ("code", string_schema()),
                ("message", string_schema()),
            ]),
        ),
        ("exitCode", integer_schema()),
        (
            "meta",
            object_schema(vec![
                ("mode", string_schema()),
                ("runId", string_schema()),
                ("schemaVersion", string_schema()),
                ("source", string_schema()),
                ("fetchedAt", string_schema()),
                ("staleAfter", string_schema()),
                ("timestamp", string_schema()),
                ("version", string_schema()),
            ]),
        ),
        ("ok", boolean_schema()),
        (
            "target",
            object_schema(vec![("kind", string_schema()), ("name", string_schema())]),
        ),
        (
            "warnings",
            array_schema(object_schema(vec![
                ("code", string_schema()),
                ("message", string_schema()),
            ])),
        ),
    ]);
    let ndjson_event_schema = object_schema(vec![
        ("action", string_schema()),
        ("data", object_schema(Vec::new())),
        (
            "meta",
            object_schema(vec![
                ("mode", string_schema()),
                ("runId", string_schema()),
                ("schemaVersion", string_schema()),
                ("source", string_schema()),
                ("fetchedAt", string_schema()),
                ("staleAfter", string_schema()),
                ("timestamp", string_schema()),
                ("version", string_schema()),
            ]),
        ),
        (
            "target",
            object_schema(vec![("kind", string_schema()), ("name", string_schema())]),
        ),
        ("type", string_schema()),
    ]);

    vec![
        SchemaDocument {
            data_schema: object_schema(vec![
                ("agents", array_schema(string_schema())),
                ("features", feature_capabilities_schema()),
                ("installers", installer_capabilities_schema()),
                ("outputModes", array_schema(string_schema())),
                ("platform", platform_capabilities_schema()),
            ]),
            description: "Environment and surface capabilities",
            envelope_schema: envelope_schema.clone(),
            name: "capabilities",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: object_schema(vec![(
                "commands",
                array_schema(command_descriptor_schema()),
            )]),
            description: "Stable command catalog",
            envelope_schema: envelope_schema.clone(),
            name: "commands",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: object_schema(vec![
                ("action", string_schema()),
                ("config", config_values_schema()),
                ("key", string_schema()),
                ("value", scalar_or_object_schema()),
            ]),
            description: "Configuration state or mutation result",
            envelope_schema: envelope_schema.clone(),
            name: "config",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: object_schema(vec![
                (
                    "agents",
                    array_schema(object_schema(vec![
                        ("displayName", string_schema()),
                        ("installedVersion", string_schema()),
                        ("latestVersion", string_schema()),
                        ("lifecycle", string_schema()),
                        ("outdated", boolean_schema()),
                        ("sourceLabel", string_schema()),
                    ])),
                ),
                (
                    "checks",
                    array_schema(object_schema(vec![
                        ("name", string_schema()),
                        ("detail", string_schema()),
                        ("recoveryHint", string_schema()),
                        ("status", string_schema()),
                    ])),
                ),
                (
                    "installSource",
                    object_schema(vec![
                        ("kind", string_schema()),
                        ("confidence", string_schema()),
                        ("executable", string_schema()),
                        ("recorded", string_schema()),
                    ]),
                ),
                (
                    "installers",
                    object_schema(vec![
                        ("brew", boolean_schema()),
                        ("bun", boolean_schema()),
                        ("npm", boolean_schema()),
                        ("winget", boolean_schema()),
                    ]),
                ),
                (
                    "issues",
                    array_schema(object_schema(vec![
                        ("blocking", boolean_schema()),
                        ("category", string_schema()),
                        ("code", string_schema()),
                        ("docsRef", string_schema()),
                        ("message", string_schema()),
                        ("severity", string_schema()),
                        (
                            "subject",
                            object_schema(vec![
                                ("kind", string_schema()),
                                ("name", string_schema()),
                            ]),
                        ),
                        ("suggestedAction", string_schema()),
                        ("suggestedCommands", array_schema(string_schema())),
                    ])),
                ),
                ("ok", boolean_schema()),
                (
                    "paths",
                    object_schema(vec![
                        ("configFile", string_schema()),
                        ("executable", string_schema()),
                        ("stateFile", string_schema()),
                    ]),
                ),
                (
                    "self",
                    object_schema(vec![
                        ("canAutoUpdate", boolean_schema()),
                        ("currentVersion", string_schema()),
                        ("installSource", string_schema()),
                        ("latestVersion", string_schema()),
                        ("outdated", boolean_schema()),
                        ("recoveryHint", string_schema()),
                    ]),
                ),
                ("summary", string_schema()),
            ]),
            description: "AGX runtime diagnostics",
            envelope_schema: envelope_schema.clone(),
            name: "doctor",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: lifecycle_data_schema(),
            description: "Ensure result for an agent",
            envelope_schema: envelope_schema.clone(),
            name: "ensure",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: object_schema(vec![
                (
                    "agent",
                    object_schema(vec![
                        ("displayName", string_schema()),
                        ("name", string_schema()),
                    ]),
                ),
                ("execution", exec_execution_schema()),
            ]),
            description: "Agent execution result",
            envelope_schema: envelope_schema.clone(),
            name: "exec",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: object_schema(vec![
                ("agent", agent_info_schema()),
                ("inspection", agent_inspection_schema()),
            ]),
            description: "Agent details",
            envelope_schema: envelope_schema.clone(),
            name: "info",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: object_schema(vec![
                ("agent", agent_info_schema()),
                ("capabilities", agent_capabilities_schema()),
                ("inspection", agent_inspection_schema()),
            ]),
            description: "Structured inspection result for an agent",
            envelope_schema: envelope_schema.clone(),
            name: "inspect",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: install_data_schema(),
            description: "Install result for one or more agents",
            envelope_schema: envelope_schema.clone(),
            name: "install",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: object_schema(vec![("agents", array_schema(listed_agent_schema()))]),
            description: "Supported agent catalog",
            envelope_schema: envelope_schema.clone(),
            name: "list",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: object_schema(vec![
                ("agent", agent_info_schema()),
                ("resolution", resolution_schema()),
            ]),
            description: "Resolved executable entrypoint for an agent",
            envelope_schema: envelope_schema.clone(),
            name: "resolve",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: object_schema(vec![("commands", array_schema(schema_document_schema()))]),
            description: "Structured schema catalog",
            envelope_schema: envelope_schema.clone(),
            name: "schema",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: lifecycle_data_schema(),
            description: "Uninstall result for an agent",
            envelope_schema: envelope_schema.clone(),
            name: "uninstall",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: object_schema(vec![
                ("channel", string_schema()),
                ("command", array_schema(string_schema())),
                ("currentVersion", string_schema()),
                ("dryRun", boolean_schema()),
                ("installSource", string_schema()),
                ("latestVersion", string_schema()),
                ("message", string_schema()),
                ("packageName", string_schema()),
                ("status", string_schema()),
                ("verifiedVersion", string_schema()),
            ]),
            description: "AGX self-upgrade result",
            envelope_schema: envelope_schema.clone(),
            name: "upgrade",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: object_schema(vec![
                ("results", array_schema(update_result_schema())),
                ("scope", string_schema()),
            ]),
            description: "Update results for one or all agents",
            envelope_schema,
            name: "update",
            ndjson_event_schema,
        },
    ]
}

fn lifecycle_data_schema() -> JsonSchema {
    object_schema(vec![
        ("agent", lifecycle_agent_schema()),
        ("changed", boolean_schema()),
        ("installState", installed_agent_state_schema()),
        ("installed", boolean_schema()),
        ("message", string_schema()),
    ])
}

fn install_data_schema() -> JsonSchema {
    object_schema(vec![
        ("agent", lifecycle_agent_schema()),
        ("changed", boolean_schema()),
        ("installState", installed_agent_state_schema()),
        ("installed", boolean_schema()),
        ("message", string_schema()),
        (
            "results",
            array_schema(lifecycle_batch_result_item_schema()),
        ),
        ("scope", string_schema()),
        ("summary", lifecycle_batch_summary_schema()),
    ])
}

fn schema_document_schema() -> JsonSchema {
    object_schema(vec![
        ("dataSchema", json_schema_schema()),
        ("description", string_schema()),
        ("envelopeSchema", json_schema_schema()),
        ("name", string_schema()),
        ("ndjsonEventSchema", json_schema_schema()),
    ])
}

fn json_schema_schema() -> JsonSchema {
    object_schema(vec![
        ("additionalProperties", boolean_schema()),
        ("items", object_schema(Vec::new())),
        ("properties", array_schema(schema_property_schema())),
        ("required", array_schema(string_schema())),
        ("type", string_schema()),
    ])
}

fn schema_property_schema() -> JsonSchema {
    object_schema(vec![
        ("name", string_schema()),
        ("schema", object_schema(Vec::new())),
    ])
}

fn command_descriptor_schema() -> JsonSchema {
    object_schema(vec![
        ("flags", array_schema(string_schema())),
        ("name", string_schema()),
        ("outputSchemaRef", string_schema()),
        ("stability", string_schema()),
        ("summary", string_schema()),
    ])
}

fn feature_capabilities_schema() -> JsonSchema {
    object_schema(vec![
        ("assumeYes", boolean_schema()),
        ("cacheBypass", boolean_schema()),
        ("cacheRefresh", boolean_schema()),
        ("channels", array_schema(string_schema())),
        ("colorModes", array_schema(string_schema())),
        ("dryRun", boolean_schema()),
        ("execInstallPolicies", array_schema(string_schema())),
        ("freshnessMetadata", boolean_schema()),
        ("idempotencyKey", boolean_schema()),
        ("logLevels", array_schema(string_schema())),
        ("quietLogs", boolean_schema()),
        ("selfUpgrade", boolean_schema()),
        ("timeout", boolean_schema()),
    ])
}

fn installer_availability_schema() -> JsonSchema {
    object_schema(vec![
        ("available", boolean_schema()),
        ("reason", string_schema()),
    ])
}

fn installer_capabilities_schema() -> JsonSchema {
    object_schema(vec![
        ("brew", installer_availability_schema()),
        ("bun", installer_availability_schema()),
        ("npm", installer_availability_schema()),
        ("winget", installer_availability_schema()),
    ])
}

fn platform_capabilities_schema() -> JsonSchema {
    object_schema(vec![("arch", string_schema()), ("os", string_schema())])
}

fn config_values_schema() -> JsonSchema {
    object_schema(vec![
        ("defaultPackageManager", string_schema()),
        ("networkRetries", integer_schema()),
        ("networkTimeoutMs", integer_schema()),
        ("npmBunUpdateStrategy", string_schema()),
        ("selfUpdateChannel", string_schema()),
        ("selfUpdateRegistry", string_schema()),
        ("versionCacheTtlHours", integer_schema()),
    ])
}

fn scalar_or_object_schema() -> JsonSchema {
    object_schema(Vec::new())
}

fn install_method_schema() -> JsonSchema {
    object_schema(vec![
        ("command", string_schema()),
        ("label", string_schema()),
        ("type", string_schema()),
    ])
}

fn agent_info_schema() -> JsonSchema {
    object_schema(vec![
        ("aliases", array_schema(string_schema())),
        ("binaryName", string_schema()),
        ("displayName", string_schema()),
        ("homepage", string_schema()),
        ("installMethods", array_schema(install_method_schema())),
        ("name", string_schema()),
        ("packageName", string_schema()),
        ("selfUpdateCommands", array_schema(string_schema())),
    ])
}

fn agent_inspection_schema() -> JsonSchema {
    object_schema(vec![
        ("binaryPath", string_schema()),
        ("installed", boolean_schema()),
        ("installedVersion", string_schema()),
        ("latestVersion", string_schema()),
        ("lifecycle", string_schema()),
        ("sourceLabel", string_schema()),
        ("updateLabel", string_schema()),
    ])
}

fn agent_capabilities_schema() -> JsonSchema {
    object_schema(vec![
        ("canAutoInstall", boolean_schema()),
        ("canAutoUninstall", boolean_schema()),
        ("canRun", boolean_schema()),
        ("canSelfUpdate", boolean_schema()),
        ("installMethods", array_schema(install_method_schema())),
        ("selfUpdateCommands", array_schema(string_schema())),
    ])
}

fn listed_agent_schema() -> JsonSchema {
    object_schema(vec![
        ("binaryName", string_schema()),
        ("displayName", string_schema()),
        ("installed", boolean_schema()),
        ("installedVersion", string_schema()),
        ("latestVersion", string_schema()),
        ("lifecycle", string_schema()),
        ("name", string_schema()),
        ("sourceLabel", string_schema()),
        ("updateLabel", string_schema()),
    ])
}

fn install_guidance_schema() -> JsonSchema {
    object_schema(vec![
        ("docsRef", string_schema()),
        ("installMethods", array_schema(install_method_schema())),
        ("suggestedAction", string_schema()),
        ("suggestedEnsureCommand", string_schema()),
    ])
}

fn resolution_schema() -> JsonSchema {
    object_schema(vec![
        ("binaryPath", string_schema()),
        ("installGuidance", install_guidance_schema()),
        ("installed", boolean_schema()),
        ("installSource", string_schema()),
        ("lifecycle", string_schema()),
        ("sourceLabel", string_schema()),
        ("suggestedLaunchCommand", array_schema(string_schema())),
    ])
}

fn exec_install_guidance_schema() -> JsonSchema {
    object_schema(vec![
        ("docsRef", string_schema()),
        ("installMethods", array_schema(install_method_schema())),
        ("suggestedAction", string_schema()),
        ("suggestedEnsureCommand", string_schema()),
        ("suggestedExecCommand", string_schema()),
    ])
}

fn exec_execution_schema() -> JsonSchema {
    object_schema(vec![
        ("args", array_schema(string_schema())),
        ("binaryPath", string_schema()),
        ("command", array_schema(string_schema())),
        ("dryRun", boolean_schema()),
        ("exitCode", integer_schema()),
        ("installGuidance", exec_install_guidance_schema()),
        ("installPolicy", string_schema()),
        ("installedAfter", boolean_schema()),
        ("installedBefore", boolean_schema()),
        ("message", string_schema()),
        ("stderr", string_schema()),
        ("stdout", string_schema()),
    ])
}

fn lifecycle_agent_schema() -> JsonSchema {
    object_schema(vec![
        ("displayName", string_schema()),
        ("name", string_schema()),
    ])
}

fn installed_agent_state_schema() -> JsonSchema {
    object_schema(vec![
        ("agentName", string_schema()),
        ("command", string_schema()),
        ("installType", string_schema()),
        ("packageName", string_schema()),
        ("packageTargetKind", string_schema()),
    ])
}

fn update_result_schema() -> JsonSchema {
    object_schema(vec![
        ("displayName", string_schema()),
        ("hint", string_schema()),
        ("installedVersion", string_schema()),
        ("latestVersion", string_schema()),
        ("message", string_schema()),
        ("name", string_schema()),
        ("resource", string_schema()),
        ("status", string_schema()),
        ("strategy", string_schema()),
    ])
}

fn lifecycle_batch_result_item_schema() -> JsonSchema {
    object_schema(vec![
        ("agent", lifecycle_agent_schema()),
        ("changed", boolean_schema()),
        (
            "error",
            object_schema(vec![
                ("code", string_schema()),
                ("message", string_schema()),
            ]),
        ),
        ("input", string_schema()),
        ("installState", installed_agent_state_schema()),
        ("installed", boolean_schema()),
        ("ok", boolean_schema()),
        ("status", string_schema()),
        (
            "warnings",
            array_schema(object_schema(vec![
                ("code", string_schema()),
                ("message", string_schema()),
            ])),
        ),
    ])
}

fn lifecycle_batch_summary_schema() -> JsonSchema {
    object_schema(vec![
        ("alreadyInstalled", integer_schema()),
        ("failed", integer_schema()),
        ("installed", integer_schema()),
        ("locked", integer_schema()),
        ("planned", integer_schema()),
        ("trackedExistingInstall", integer_schema()),
        ("untrackedExistingInstall", integer_schema()),
    ])
}

fn agent_info(agent: AgentDefinition) -> AgentInfo {
    AgentInfo {
        aliases: agent.aliases.to_vec(),
        binary_name: agent.binary_name,
        display_name: agent.display_name,
        homepage: agent.homepage,
        install_methods: install_methods(agent),
        name: agent.name,
        package_name: agent.npm_package,
        self_update_commands: agents::self_update_commands(agent),
    }
}

fn agent_capabilities(agent: AgentDefinition) -> AgentCapabilities {
    let inspection = resolved_agent_inspection(
        agent,
        &CliContext {
            assume_yes: false,
            cache_mode: crate::context::CacheMode::Default,
            color_mode: crate::context::ColorMode::Never,
            dry_run: false,
            idempotency_key: None,
            interactive: false,
            log_level: crate::context::LogLevel::Silent,
            output_mode: crate::context::OutputMode::Json,
            quiet: true,
            run_id: "agent-capabilities".to_string(),
            timeout_ms: None,
        },
    );
    let self_update_commands = agents::self_update_commands(agent);
    let can_self_update = !self_update_commands.is_empty();

    AgentCapabilities {
        can_auto_install: agent.npm_package.is_some(),
        can_auto_uninstall: inspection.installed && inspection.lifecycle == "managed",
        can_run: inspection.installed,
        can_self_update,
        install_methods: install_methods(agent),
        self_update_commands,
    }
}

fn resolved_agent_inspection(
    agent: AgentDefinition,
    context: &CliContext,
) -> inspection::AgentInspection {
    let mut inspection = inspection::inspect_agent(agent, context);
    let installed_state = crate::state::get_installed_agent_state(agent.name);

    inspection.lifecycle = lifecycle_for(installed_state.as_ref()).to_string();
    inspection.source_label = source_label_for(installed_state.as_ref(), inspection.installed);
    inspection.update_label = update_label_for(agent, installed_state.as_ref());

    inspection
}

fn install_methods(agent: AgentDefinition) -> Vec<InstallMethodInfo> {
    agent.npm_package.map_or_else(Vec::new, |package| {
        vec![
            InstallMethodInfo {
                command: format!("bun add -g {package}"),
                label: "bun",
                method_type: "bun",
            },
            InstallMethodInfo {
                command: format!("npm install -g {package}"),
                label: "npm",
                method_type: "npm",
            },
        ]
    })
}

fn lifecycle_for(installed_state: Option<&crate::state::InstalledAgentState>) -> &'static str {
    if installed_state.is_some_and(|state| is_managed_install_type(&state.install_type)) {
        "managed"
    } else {
        "unmanaged"
    }
}

fn install_source_for(agent: AgentDefinition) -> &'static str {
    crate::state::get_installed_agent_state(agent.name)
        .map_or("path", |state| install_source_kind(&state.install_type))
}

fn install_source_kind(install_type: &str) -> &'static str {
    match install_type {
        "bun" => "bun",
        "npm" => "npm",
        "brew" => "brew",
        "winget" => "winget",
        "script" => "script",
        "binary" => "binary",
        _ => "path",
    }
}

fn source_label_for(
    installed_state: Option<&crate::state::InstalledAgentState>,
    installed: bool,
) -> String {
    if let Some(state) = installed_state {
        return if is_managed_install_type(&state.install_type) {
            format!(
                "managed via {}{}",
                state.install_type,
                format_package_target(
                    state.package_name.as_deref(),
                    state.package_target_kind.as_deref()
                )
            )
        } else if state.install_type == "script" {
            "script installer".to_string()
        } else {
            "binary installer".to_string()
        };
    }

    if installed {
        "detected in PATH".to_string()
    } else {
        "untracked".to_string()
    }
}

fn update_label_for(
    agent: AgentDefinition,
    installed_state: Option<&crate::state::InstalledAgentState>,
) -> String {
    if installed_state.is_some_and(|state| is_managed_install_type(&state.install_type)) {
        "managed update".to_string()
    } else if !agents::self_update_commands(agent).is_empty() {
        "command update".to_string()
    } else {
        "manual update".to_string()
    }
}

fn is_managed_install_type(install_type: &str) -> bool {
    matches!(install_type, "bun" | "npm" | "brew" | "winget")
}

fn format_package_target(package_name: Option<&str>, package_target_kind: Option<&str>) -> String {
    let Some(package_name) = package_name else {
        return String::new();
    };

    match package_target_kind {
        Some("cask") => format!(" ({package_name} cask)"),
        Some("id") => format!(" ({package_name} id)"),
        _ => format!(" ({package_name})"),
    }
}

fn agent_not_found_result(
    action: &'static str,
    agent_name: &str,
    context: &CliContext,
) -> CommandResult {
    CommandResult::error(
        action,
        AgxError::new(
            AgxErrorCode::AgentNotFound,
            format!("Unknown agent: {agent_name}"),
        ),
        CommandTarget::agent(agent_name),
        context,
    )
}

fn invalid_config_argument(
    message: impl Into<String>,
    key: Option<String>,
    context: &CliContext,
) -> CommandResult {
    CommandResult::error(
        "config",
        AgxError::new(AgxErrorCode::InvalidArgument, message),
        CommandTarget::config(key),
        context,
    )
}

fn exec_missing_result(
    agent: AgentDefinition,
    args: &[String],
    install_policy: crate::cli::InstallPolicyArg,
    context: &CliContext,
    error_code: AgxErrorCode,
    message: String,
) -> CommandResult {
    CommandResult::error_with_data(
        "exec",
        ExecCommandData {
            agent: exec::ExecAgent {
                display_name: agent.display_name,
                name: agent.name,
            },
            execution: exec::ExecExecution {
                args: args.to_vec(),
                binary_path: None,
                command: std::iter::once(agent.binary_name.to_string())
                    .chain(args.iter().cloned())
                    .collect(),
                dry_run: false,
                exit_code: None,
                install_policy: match install_policy {
                    crate::cli::InstallPolicyArg::Never => "never",
                    crate::cli::InstallPolicyArg::IfMissing => "if-missing",
                    crate::cli::InstallPolicyArg::Always => "always",
                },
                install_guidance: Some(exec::install_guidance(agent, args)),
                installed_after: false,
                installed_before: false,
                message: Some(message.clone()),
                stderr: None,
                stdout: None,
            },
        },
        AgxError::new(error_code, message),
        CommandTarget::agent(agent.name),
        context,
    )
}

fn dry_run_warning() -> CommandWarning {
    CommandWarning {
        code: "DRY_RUN".to_string(),
        message: "Dry run mode is enabled; no changes were applied.".to_string(),
    }
}

fn object_schema(properties: Vec<(&'static str, JsonSchema)>) -> JsonSchema {
    JsonSchema {
        additional_properties: Some(false),
        items: None,
        properties: Some(
            properties
                .into_iter()
                .map(|(name, schema)| SchemaProperty { name, schema })
                .collect(),
        ),
        required: None,
        schema_type: "object",
    }
}

fn array_schema(items: JsonSchema) -> JsonSchema {
    JsonSchema {
        additional_properties: None,
        items: Some(Box::new(items)),
        properties: None,
        required: None,
        schema_type: "array",
    }
}

fn string_schema() -> JsonSchema {
    primitive_schema("string")
}

fn boolean_schema() -> JsonSchema {
    primitive_schema("boolean")
}

fn integer_schema() -> JsonSchema {
    primitive_schema("integer")
}

fn primitive_schema(schema_type: &'static str) -> JsonSchema {
    JsonSchema {
        additional_properties: None,
        items: None,
        properties: None,
        required: None,
        schema_type,
    }
}
