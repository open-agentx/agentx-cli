use serde::Serialize;

use crate::agents::{self, AgentDefinition};
use crate::cli::Command;
use crate::context::CliContext;
use crate::errors::{AgxError, AgxErrorCode};
use crate::output::{CommandResult, CommandTarget};

pub fn run_command(command: &Command, context: &CliContext) -> CommandResult {
    match command {
        Command::Capabilities => capabilities_command(context),
        Command::Commands => commands_command(context),
        Command::Info { agent } => info_command(agent, context),
        Command::List => list_command(context),
        Command::Schema { command } => schema_command(command.as_deref(), context),
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
    inspection: AgentInspection,
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
struct AgentInspection {
    installed: bool,
    lifecycle: &'static str,
    source_label: &'static str,
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

fn list_command(context: &CliContext) -> CommandResult {
    CommandResult::success(
        "list",
        ListData {
            agents: agents::all_agents()
                .iter()
                .map(|agent| ListedAgent {
                    binary_name: agent.binary_name,
                    display_name: agent.display_name,
                    installed: is_command_available(agent.binary_name),
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
            inspection: AgentInspection {
                installed: is_command_available(agent.binary_name),
                lifecycle: "unmanaged",
                source_label: "untracked",
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
                "--timeout",
            ],
            name: "schema",
            output_schema_ref: "#/commands/schema",
            stability: "stable",
            summary: "Return structured output schemas",
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
                ("agent", object_schema(Vec::new())),
                ("inspection", object_schema(Vec::new())),
            ]),
            description: "Agent details",
            envelope_schema: envelope_schema.clone(),
            name: "info",
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
            data_schema: object_schema(vec![("commands", array_schema(object_schema(Vec::new())))]),
            description: "Structured schema catalog",
            envelope_schema,
            name: "schema",
            ndjson_event_schema,
        },
    ]
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
