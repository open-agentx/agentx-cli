use serde::Serialize;

use crate::agents::{self, AgentDefinition};
use crate::cli::Command;
use crate::config;
use crate::context::CliContext;
use crate::errors::{AgxError, AgxErrorCode};
use crate::inspection;
use crate::output::{CommandResult, CommandTarget};
use crate::package_manager;

pub fn run_command(command: &Command, context: &CliContext) -> CommandResult {
    match command {
        Command::Capabilities => capabilities_command(context),
        Command::Commands => commands_command(context),
        Command::Config { action, key, value } => {
            config_command(action.as_deref(), key.as_deref(), value.as_deref(), context)
        }
        Command::Ensure { agent } => ensure_command(agent, context),
        Command::Info { agent } => info_command(agent, context),
        Command::Install { agent } => install_command(agent, context),
        Command::Inspect { agent } => inspect_command(agent, context),
        Command::List => list_command(context),
        Command::Resolve { agent } => resolve_command(agent, context),
        Command::Schema { command } => schema_command(command.as_deref(), context),
        Command::Uninstall { agent } => uninstall_command(agent, context),
        Command::Update { agent, all } => update_command(agent.as_deref(), *all, context),
    }
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
    lifecycle: &'static str,
    name: &'static str,
    source_label: &'static str,
    update_label: &'static str,
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
struct LifecycleAgent {
    display_name: &'static str,
    name: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateData {
    results: Vec<package_manager::UpdateResult>,
    scope: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentInfo {
    aliases: Vec<&'static str>,
    binary_name: &'static str,
    display_name: &'static str,
    homepage: &'static str,
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
struct AgentCapabilities {
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
    lifecycle: &'static str,
    source_label: &'static str,
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
                .map(|agent| ListedAgent {
                    binary_name: agent.binary_name,
                    display_name: agent.display_name,
                    installed: inspection::find_binary_in_path(agent.binary_name).is_some(),
                    lifecycle: "unmanaged",
                    name: agent.name,
                    source_label: "untracked",
                    update_label: "manual",
                })
                .collect(),
        },
        CommandTarget::system("agents"),
        context,
    )
}

fn install_command(agent_name: &str, context: &CliContext) -> CommandResult {
    lifecycle_command(
        "install",
        agent_name,
        context,
        package_manager::install_agent,
    )
}

fn ensure_command(agent_name: &str, context: &CliContext) -> CommandResult {
    lifecycle_command("ensure", agent_name, context, package_manager::ensure_agent)
}

fn uninstall_command(agent_name: &str, context: &CliContext) -> CommandResult {
    lifecycle_command(
        "uninstall",
        agent_name,
        context,
        package_manager::uninstall_agent,
    )
}

fn update_command(agent_name: Option<&str>, all: bool, context: &CliContext) -> CommandResult {
    if all {
        let results = package_manager::update_all_agents(context);
        let has_failures = results
            .iter()
            .any(|result| matches!(result.status, "failed" | "locked"));
        let data = UpdateData {
            results,
            scope: "all",
        };
        if has_failures {
            return CommandResult::error(
                "update",
                AgxError::new(
                    AgxErrorCode::UpdateFailed,
                    "One or more agents failed to update.",
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

    match package_manager::update_agent(agent, context) {
        Ok(result) => CommandResult::success(
            "update",
            UpdateData {
                results: vec![result],
                scope: "single",
            },
            CommandTarget::agent(agent.name),
            context,
        ),
        Err(error) => {
            CommandResult::error("update", error, CommandTarget::agent(agent.name), context)
        }
    }
}

fn info_command(agent_name: &str, context: &CliContext) -> CommandResult {
    let Some(agent) = agents::resolve_agent(agent_name) else {
        return CommandResult::error(
            "info",
            AgxError::new(
                AgxErrorCode::AgentNotFound,
                format!("Unknown agent: {agent_name}"),
            ),
            CommandTarget::agent(agent_name),
            context,
        );
    };

    CommandResult::success(
        "info",
        InfoData {
            agent: agent_info(agent),
            inspection: inspection::inspect_agent(agent),
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
            capabilities: AgentCapabilities {
                install_methods: install_methods(agent),
                self_update_commands: Vec::new(),
            },
            inspection: inspection::inspect_agent(agent),
        },
        CommandTarget::agent(agent.name),
        context,
    )
}

fn resolve_command(agent_name: &str, context: &CliContext) -> CommandResult {
    let Some(agent) = agents::resolve_agent(agent_name) else {
        return agent_not_found_result("resolve", agent_name, context);
    };

    let inspection = inspection::inspect_agent(agent);
    let install_methods = install_methods(agent);
    let installed = inspection.installed;
    let install_guidance = if installed {
        None
    } else {
        Some(InstallGuidance {
            docs_ref: "openspec/changes/rewrite-quantex-cli-as-agx-rust/tasks.md",
            install_methods,
            suggested_action: "ensure-agent-installed",
            suggested_ensure_command: format!("agx ensure {}", agent.name),
        })
    };

    CommandResult::success(
        "resolve",
        ResolveData {
            agent: agent_info(agent),
            resolution: Resolution {
                binary_path: inspection.binary_path,
                install_guidance,
                installed,
                install_source: "untracked",
                lifecycle: "unmanaged",
                source_label: "untracked",
                suggested_launch_command: vec![agent.binary_name.to_string()],
            },
        },
        CommandTarget::agent(agent.name),
        context,
    )
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
    let Some(agent) = agents::resolve_agent(agent_name) else {
        return agent_not_found_result(action, agent_name, context);
    };

    match operation(agent, context) {
        Ok(result) => CommandResult::success(
            action,
            LifecycleData {
                agent: LifecycleAgent {
                    display_name: agent.display_name,
                    name: agent.name,
                },
                changed: result.changed,
                install_state: result.install_state,
                installed: result.installed,
                message: result.message,
            },
            CommandTarget::agent(agent.name),
            context,
        ),
        Err(error) => {
            CommandResult::error(action, error, CommandTarget::agent(agent.name), context)
        }
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
            summary: "Install an agent",
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
        ("error", object_schema(Vec::new())),
        ("exitCode", integer_schema()),
        ("meta", object_schema(Vec::new())),
        ("ok", boolean_schema()),
        ("target", object_schema(Vec::new())),
        ("warnings", array_schema(object_schema(Vec::new()))),
    ]);
    let ndjson_event_schema = object_schema(vec![
        ("action", string_schema()),
        ("data", object_schema(Vec::new())),
        ("meta", object_schema(Vec::new())),
        ("target", object_schema(Vec::new())),
        ("type", string_schema()),
    ]);

    vec![
        SchemaDocument {
            data_schema: object_schema(vec![
                ("agents", array_schema(string_schema())),
                ("features", object_schema(Vec::new())),
                ("installers", object_schema(Vec::new())),
                ("outputModes", array_schema(string_schema())),
                ("platform", object_schema(Vec::new())),
            ]),
            description: "Environment and surface capabilities",
            envelope_schema: envelope_schema.clone(),
            name: "capabilities",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: object_schema(vec![("commands", array_schema(object_schema(Vec::new())))]),
            description: "Stable command catalog",
            envelope_schema: envelope_schema.clone(),
            name: "commands",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: object_schema(vec![
                ("action", string_schema()),
                ("config", object_schema(Vec::new())),
                ("key", string_schema()),
                ("value", object_schema(Vec::new())),
            ]),
            description: "Configuration state or mutation result",
            envelope_schema: envelope_schema.clone(),
            name: "config",
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
                ("agent", object_schema(Vec::new())),
                ("inspection", object_schema(Vec::new())),
            ]),
            description: "Agent details",
            envelope_schema: envelope_schema.clone(),
            name: "info",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: object_schema(vec![
                ("agent", object_schema(Vec::new())),
                ("capabilities", object_schema(Vec::new())),
                ("inspection", object_schema(Vec::new())),
            ]),
            description: "Structured inspection result for an agent",
            envelope_schema: envelope_schema.clone(),
            name: "inspect",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: lifecycle_data_schema(),
            description: "Install result for an agent",
            envelope_schema: envelope_schema.clone(),
            name: "install",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: object_schema(vec![("agents", array_schema(object_schema(Vec::new())))]),
            description: "Supported agent catalog",
            envelope_schema: envelope_schema.clone(),
            name: "list",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: object_schema(vec![
                ("agent", object_schema(Vec::new())),
                ("resolution", object_schema(Vec::new())),
            ]),
            description: "Resolved executable entrypoint for an agent",
            envelope_schema: envelope_schema.clone(),
            name: "resolve",
            ndjson_event_schema: ndjson_event_schema.clone(),
        },
        SchemaDocument {
            data_schema: object_schema(vec![("commands", array_schema(object_schema(Vec::new())))]),
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
                ("results", array_schema(object_schema(Vec::new()))),
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
        ("agent", object_schema(Vec::new())),
        ("changed", boolean_schema()),
        ("installState", object_schema(Vec::new())),
        ("installed", boolean_schema()),
    ])
}

fn agent_info(agent: AgentDefinition) -> AgentInfo {
    AgentInfo {
        aliases: agent.aliases.to_vec(),
        binary_name: agent.binary_name,
        display_name: agent.display_name,
        homepage: agent.homepage,
        name: agent.name,
        package_name: agent.npm_package,
        self_update_commands: Vec::new(),
    }
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
