use serde::Serialize;
use serde_json::Value;

use nu_ansi_term::{Color, Style};

use crate::context::{CliContext, ColorMode, OutputMode};
use crate::errors::{AgxError, AgxErrorCode};

const SCHEMA_VERSION: &str = "1";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandTarget {
    pub kind: TargetKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl CommandTarget {
    pub fn agent(name: impl Into<String>) -> Self {
        Self {
            kind: TargetKind::Agent,
            name: Some(name.into()),
        }
    }

    pub fn config(name: Option<String>) -> Self {
        Self {
            kind: TargetKind::Config,
            name,
        }
    }

    pub fn system(name: impl Into<String>) -> Self {
        Self {
            kind: TargetKind::System,
            name: Some(name.into()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub enum TargetKind {
    Agent,
    Config,
    #[serde(rename = "self")]
    SelfTarget,
    System,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: AgxErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<String>,
    pub mode: &'static str,
    pub run_id: String,
    pub schema_version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<crate::context::FreshnessSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_after: Option<String>,
    pub timestamp: String,
    pub version: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    pub error: Option<CommandError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<u8>,
    pub meta: CommandMeta,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<CommandTarget>,
    pub warnings: Vec<CommandWarning>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NdjsonEvent<T> {
    action: String,
    data: T,
    meta: CommandMeta,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<CommandTarget>,
    #[serde(rename = "type")]
    kind: &'static str,
}

impl CommandResult {
    pub fn success(
        action: impl Into<String>,
        data: impl Serialize,
        target: CommandTarget,
        context: &CliContext,
    ) -> Self {
        Self {
            action: action.into(),
            data: Some(serde_json::to_value(data).expect("command data must serialize")),
            error: None,
            exit_code: None,
            meta: create_meta(context),
            ok: true,
            target: Some(target),
            warnings: Vec::new(),
        }
    }

    pub fn success_with_exit_code(
        action: impl Into<String>,
        data: impl Serialize,
        target: CommandTarget,
        context: &CliContext,
        exit_code: u8,
    ) -> Self {
        Self {
            action: action.into(),
            data: Some(serde_json::to_value(data).expect("command data must serialize")),
            error: None,
            exit_code: Some(exit_code),
            meta: create_meta(context),
            ok: true,
            target: Some(target),
            warnings: Vec::new(),
        }
    }

    pub fn error(
        action: impl Into<String>,
        error: AgxError,
        target: CommandTarget,
        context: &CliContext,
    ) -> Self {
        Self::error_with_details(action, error, None, target, context)
    }

    pub fn error_with_details(
        action: impl Into<String>,
        error: AgxError,
        details: Option<Value>,
        target: CommandTarget,
        context: &CliContext,
    ) -> Self {
        let exit_code = error.exit_code();
        Self {
            action: action.into(),
            data: None,
            error: Some(CommandError {
                code: error.code,
                details,
                message: error.message,
            }),
            exit_code: Some(exit_code),
            meta: create_meta(context),
            ok: false,
            target: Some(target),
            warnings: Vec::new(),
        }
    }

    pub fn error_with_exit_code(
        action: impl Into<String>,
        error: AgxError,
        target: CommandTarget,
        context: &CliContext,
        exit_code: u8,
    ) -> Self {
        Self {
            action: action.into(),
            data: None,
            error: Some(CommandError {
                code: error.code,
                details: None,
                message: error.message,
            }),
            exit_code: Some(exit_code),
            meta: create_meta(context),
            ok: false,
            target: Some(target),
            warnings: Vec::new(),
        }
    }

    pub fn error_with_data(
        action: impl Into<String>,
        data: impl Serialize,
        error: AgxError,
        target: CommandTarget,
        context: &CliContext,
    ) -> Self {
        Self::error_with_data_and_details(action, data, error, None, target, context)
    }

    pub fn error_with_data_and_details(
        action: impl Into<String>,
        data: impl Serialize,
        error: AgxError,
        details: Option<Value>,
        target: CommandTarget,
        context: &CliContext,
    ) -> Self {
        let exit_code = error.exit_code();
        Self {
            action: action.into(),
            data: Some(serde_json::to_value(data).expect("command data must serialize")),
            error: Some(CommandError {
                code: error.code,
                details,
                message: error.message,
            }),
            exit_code: Some(exit_code),
            meta: create_meta(context),
            ok: false,
            target: Some(target),
            warnings: Vec::new(),
        }
    }

    pub fn exit_code(&self) -> u8 {
        self.exit_code.unwrap_or_else(|| {
            if self.ok {
                0
            } else {
                self.error
                    .as_ref()
                    .map_or(1, |error| error.code.exit_code())
            }
        })
    }
}

pub fn emit_result(result: &CommandResult, context: &CliContext) -> Result<(), serde_json::Error> {
    match context.output_mode {
        OutputMode::Human => {
            render_human(result, &Painter::new(context.color_mode));
            Ok(())
        }
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(result)?);
            Ok(())
        }
        OutputMode::Ndjson => {
            println!(
                "{}",
                serde_json::to_string(&create_ndjson_event(result, context))?
            );
            Ok(())
        }
    }
}

pub fn emit_ndjson_event(
    action: &str,
    kind: &'static str,
    data: impl Serialize,
    target: Option<CommandTarget>,
    context: &CliContext,
) -> Result<(), serde_json::Error> {
    if !matches!(context.output_mode, OutputMode::Ndjson) {
        return Ok(());
    }

    let freshness = context
        .freshness
        .lock()
        .expect("freshness mutex should not be poisoned")
        .clone();
    println!(
        "{}",
        serde_json::to_string(&NdjsonEvent {
            action: action.to_string(),
            data,
            meta: CommandMeta {
                fetched_at: freshness
                    .as_ref()
                    .map(|freshness| freshness.fetched_at.clone()),
                mode: "ndjson",
                run_id: context.run_id.clone(),
                schema_version: SCHEMA_VERSION,
                source: freshness.as_ref().map(|freshness| freshness.source),
                stale_after: freshness
                    .as_ref()
                    .map(|freshness| freshness.stale_after.clone()),
                timestamp: current_timestamp(),
                version: env!("CARGO_PKG_VERSION"),
            },
            target,
            kind,
        })?
    );
    Ok(())
}

fn create_meta(context: &CliContext) -> CommandMeta {
    let freshness = context
        .freshness
        .lock()
        .expect("freshness mutex should not be poisoned")
        .clone();
    CommandMeta {
        fetched_at: freshness
            .as_ref()
            .map(|freshness| freshness.fetched_at.clone()),
        mode: context.output_mode.as_str(),
        run_id: context.run_id.clone(),
        schema_version: SCHEMA_VERSION,
        source: freshness.as_ref().map(|freshness| freshness.source),
        stale_after: freshness
            .as_ref()
            .map(|freshness| freshness.stale_after.clone()),
        timestamp: current_timestamp(),
        version: env!("CARGO_PKG_VERSION"),
    }
}

fn create_ndjson_event<'a>(result: &'a CommandResult, context: &CliContext) -> impl Serialize + 'a {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Event<'a> {
        action: &'a str,
        data: &'a CommandResult,
        meta: CommandMeta,
        #[serde(rename = "type")]
        kind: &'static str,
    }

    Event {
        action: &result.action,
        data: result,
        meta: {
            let freshness = context
                .freshness
                .lock()
                .expect("freshness mutex should not be poisoned")
                .clone();
            CommandMeta {
                fetched_at: freshness
                    .as_ref()
                    .map(|freshness| freshness.fetched_at.clone()),
                mode: "ndjson",
                run_id: context.run_id.clone(),
                schema_version: SCHEMA_VERSION,
                source: freshness.as_ref().map(|freshness| freshness.source),
                stale_after: freshness
                    .as_ref()
                    .map(|freshness| freshness.stale_after.clone()),
                timestamp: current_timestamp(),
                version: env!("CARGO_PKG_VERSION"),
            }
        },
        kind: "result",
    }
}

struct Painter {
    enabled: bool,
}

impl Painter {
    fn new(color_mode: ColorMode) -> Self {
        Self {
            enabled: color_mode.enabled(),
        }
    }

    fn paint<'a>(&self, style: Style, value: &'a str) -> String {
        if self.enabled {
            style.paint(value).to_string()
        } else {
            value.to_string()
        }
    }

    fn bold(&self, value: &str) -> String {
        self.paint(Style::new().bold(), value)
    }

    fn cyan(&self, value: &str) -> String {
        self.paint(Color::Cyan.normal(), value)
    }

    fn dim(&self, value: &str) -> String {
        self.paint(Style::new().dimmed(), value)
    }

    fn green(&self, value: &str) -> String {
        self.paint(Color::Green.normal(), value)
    }

    fn red(&self, value: &str) -> String {
        self.paint(Color::Red.normal(), value)
    }

    fn yellow(&self, value: &str) -> String {
        self.paint(Color::Yellow.normal(), value)
    }

    fn yes(&self) -> String {
        self.green("yes")
    }

    fn no(&self) -> String {
        self.red("no")
    }
}

fn render_human(result: &CommandResult, painter: &Painter) {
    match result.action.as_str() {
        "commands" => render_commands(result, painter),
        "schema" => render_schema(result, painter),
        "capabilities" => render_capabilities(result, painter),
        "doctor" => {
            if let Some(data) = &result.data {
                render_doctor(data, painter);
            }
        }
        "ensure" => render_ensure(result, painter),
        "exec" => render_exec(result, painter),
        "install" => render_install(result, painter),
        "list" => render_list(result, painter),
        "info" => render_info(result, painter),
        "inspect" => render_inspect(result, painter),
        "resolve" => render_resolve(result, painter),
        "uninstall" => render_uninstall(result, painter),
        "upgrade" => render_upgrade(result, painter),
        "update" => render_update(result, painter),
        _ => {
            if let Some(error) = &result.error {
                eprintln!("{}", error.message);
            } else if let Some(data) = &result.data {
                render_default_human(result.action.as_str(), data);
            }
        }
    }
}

fn current_timestamp() -> String {
    use time::OffsetDateTime;
    use time::macros::format_description;

    const ISO_8601_MILLIS: &[time::format_description::FormatItem<'static>] =
        format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z");
    OffsetDateTime::now_utc()
        .format(ISO_8601_MILLIS)
        .unwrap_or_else(|_| "1970-01-01T00:00:00.000Z".to_string())
}

fn render_default_human(action: &str, data: &Value) {
    match action {
        "config" => render_config(data),
        _ => println!("{data}"),
    }
}

fn render_capabilities(result: &CommandResult, painter: &Painter) {
    let Some(data) = &result.data else {
        println!("AGX Capabilities");
        return;
    };

    println!("{}\n", painter.bold("AGX Capabilities"));
    println!(
        "Platform: {}/{}",
        data["platform"]["os"].as_str().unwrap_or("unknown"),
        data["platform"]["arch"].as_str().unwrap_or("unknown")
    );
    println!(
        "Output Modes: {}",
        data["outputModes"]
            .as_array()
            .map(|modes| {
                modes
                    .iter()
                    .filter_map(|mode| mode.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default()
    );
    println!(
        "Agents: {}",
        data["agents"]
            .as_array()
            .map(|agents| {
                agents
                    .iter()
                    .filter_map(|agent| agent.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default()
    );

    println!("\n{}:", painter.bold("Installers"));
    for installer in ["bun", "npm", "brew", "winget"] {
        let available = data["installers"][installer]["available"]
            .as_bool()
            .unwrap_or(false);
        println!(
            "  {installer}: {}",
            if available {
                painter.green("available")
            } else {
                painter.red("not found")
            }
        );
    }

    println!("\n{}:", painter.bold("Features"));
    for (label, key, mode) in [
        ("--yes", "assumeYes", "bool"),
        ("cache-refresh", "cacheRefresh", "bool"),
        ("color-modes", "colorModes", "list"),
        ("no-cache", "cacheBypass", "bool"),
        ("dry-run", "dryRun", "bool"),
        ("freshness-metadata", "freshnessMetadata", "bool"),
        ("self-upgrade", "selfUpgrade", "bool"),
        ("idempotency-key", "idempotencyKey", "bool"),
        ("log-levels", "logLevels", "list"),
        ("quiet-logs", "quietLogs", "bool"),
        ("timeout", "timeout", "bool"),
        ("channels", "channels", "list"),
        ("exec-install-policy", "execInstallPolicies", "list"),
    ] {
        let value = if mode == "list" {
            data["features"][key].as_array().map_or_else(
                || "unknown".to_string(),
                |items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                },
            )
        } else if data["features"][key].as_bool().unwrap_or(false) {
            painter.yes()
        } else {
            painter.no()
        };
        println!("  {label}: {value}");
    }
    println!();
}

fn render_commands(result: &CommandResult, painter: &Painter) {
    let Some(data) = &result.data else {
        println!("AGX Commands");
        return;
    };

    println!("{}\n", painter.bold("AGX Commands"));
    if let Some(commands) = data["commands"].as_array() {
        for command in commands {
            let name = painter.cyan(command["name"].as_str().unwrap_or("unknown"));
            let flags = command["flags"]
                .as_array()
                .map(|flags| {
                    let joined = flags
                        .iter()
                        .filter_map(|flag| flag.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    if joined.is_empty() {
                        String::new()
                    } else {
                        painter.dim(&format!(" [{joined}]"))
                    }
                })
                .unwrap_or_default();
            println!("  {}{}", name, flags);
            println!("    {}", command["summary"].as_str().unwrap_or(""));
            println!(
                "    {}",
                painter.dim(command["outputSchemaRef"].as_str().unwrap_or(""))
            );
        }
    }
    println!();
}

fn render_schema(result: &CommandResult, painter: &Painter) {
    let Some(data) = &result.data else {
        println!("AGX Schemas");
        return;
    };

    println!("{}\n", painter.bold("AGX Schemas"));
    if let Some(commands) = data["commands"].as_array() {
        for command in commands {
            println!("  {}", command["name"].as_str().unwrap_or("unknown"),);
            println!("    {}", command["description"].as_str().unwrap_or(""));
        }
    }
    println!();
}

fn render_config(data: &Value) {
    let action = data["action"].as_str().unwrap_or("list");
    match action {
        "list" | "reset" => {
            if action == "list" {
                println!("Current Configuration:\n");
            }
            if let Some(config) = data["config"].as_object()
                && let Ok(pretty) = serde_json::to_string_pretty(config)
            {
                println!("{pretty}");
            }
            if action == "reset" {
                println!("\nConfiguration reset to defaults.");
            }
            println!();
        }
        "get" => {
            if let Some(key) = data["key"].as_str() {
                let value = &data["value"];
                if value.is_null() {
                    let _ = key;
                    println!("(not set)");
                } else if let Some(string) = value.as_str() {
                    println!("{string}");
                } else {
                    println!("{value}");
                }
            }
        }
        "set" => {
            let key = data["key"].as_str().unwrap_or("unknown");
            let value = &data["value"];
            let value_text = value
                .as_str()
                .map_or_else(|| value.to_string(), ToString::to_string);
            println!("Set {key} = {value_text}");
        }
        _ => println!("{data}"),
    }
}

#[allow(clippy::too_many_lines)]
fn render_doctor(data: &Value, painter: &Painter) {
    println!("{}\n", painter.bold("AGX Environment Check"));
    println!("{}:", painter.bold("Managed Installers"));
    println!(
        "  Bun:   {}",
        if data["installers"]["bun"].as_bool().unwrap_or(false) {
            painter.green("available")
        } else {
            painter.red("not found")
        }
    );
    println!(
        "  npm:   {}",
        if data["installers"]["npm"].as_bool().unwrap_or(false) {
            painter.green("available")
        } else {
            painter.red("not found")
        }
    );
    println!(
        "  brew:  {}",
        if data["installers"]["brew"].as_bool().unwrap_or(false) {
            painter.green("available")
        } else {
            painter.red("not found")
        }
    );
    println!(
        "  winget:{}",
        if data["installers"]["winget"].as_bool().unwrap_or(false) {
            painter.green("available")
        } else {
            painter.red("not found")
        }
    );

    println!("\n{}:", painter.bold("AGX CLI"));
    println!(
        "  Version:      {}",
        data["self"]["currentVersion"].as_str().unwrap_or("unknown")
    );
    println!(
        "  Source:       {}",
        data["self"]["installSource"].as_str().unwrap_or("unknown")
    );
    println!(
        "  Auto-update:  {}",
        if data["self"]["canAutoUpdate"].as_bool().unwrap_or(false) {
            painter.green("supported")
        } else {
            painter.yellow("unsupported")
        }
    );
    if let Some(latest) = data["self"]["latestVersion"].as_str() {
        println!(
            "  Latest:       {}{}",
            latest,
            if data["self"]["outdated"].as_bool().unwrap_or(false) {
                " (update available)"
            } else {
                ""
            }
        );
    }
    if let Some(recovery) = data["self"]["recoveryHint"].as_str() {
        println!("  Recovery:     {recovery}");
    }

    println!("\n{}:", painter.bold("Installed Agents"));
    if data["agents"]
        .as_array()
        .is_some_and(std::vec::Vec::is_empty)
    {
        println!("  {}", painter.dim("No agents installed"));
    } else if let Some(agents) = data["agents"].as_array() {
        for agent in agents {
            println!(
                "  {}: {} [{}; {}]{}",
                agent["displayName"].as_str().unwrap_or("unknown"),
                agent["installedVersion"].as_str().unwrap_or("unknown"),
                agent["lifecycle"].as_str().unwrap_or("unknown"),
                agent["sourceLabel"].as_str().unwrap_or("unknown"),
                if agent["outdated"].as_bool().unwrap_or(false) {
                    painter.yellow(&format!(
                        " (update available: {})",
                        agent["latestVersion"].as_str().unwrap_or("unknown")
                    ))
                } else {
                    String::new()
                }
            );
        }
    }

    println!("\n{}:", painter.bold("Issues"));
    if data["issues"]
        .as_array()
        .is_some_and(std::vec::Vec::is_empty)
    {
        println!("  {}", painter.green("No issues found."));
    } else if let Some(issues) = data["issues"].as_array() {
        for issue in issues {
            println!(
                "  - {}",
                painter.yellow(issue["message"].as_str().unwrap_or("unknown issue"))
            );
            if let Some(commands) = issue["suggestedCommands"].as_array()
                && !commands.is_empty()
            {
                let next: Vec<_> = commands
                    .iter()
                    .filter_map(|command| command.as_str())
                    .collect();
                println!(
                    "    {}",
                    painter.dim(&format!("Next: {}", next.join(" | ")))
                );
            }
        }
    }

    println!();
}

fn render_list(result: &CommandResult, painter: &Painter) {
    println!("{}\n", painter.bold("AI Agents:"));
    if let Some(agents) = result
        .data
        .as_ref()
        .and_then(|data| data["agents"].as_array())
    {
        for agent in agents {
            let installed = agent["installed"].as_bool().unwrap_or(false);
            let name = agent["displayName"].as_str().unwrap_or("unknown");
            let name_padded = format!("{name:<18}");
            let status = if installed {
                painter.green("installed")
            } else {
                painter.dim("not installed")
            };
            let version = agent["installedVersion"].as_str().unwrap_or("");
            let version_text = if installed {
                if version.is_empty() {
                    painter.dim("unknown version")
                } else {
                    painter.dim(version)
                }
            } else {
                String::new()
            };
            let update = if installed {
                painter.cyan(agent["updateLabel"].as_str().unwrap_or(""))
            } else {
                String::new()
            };
            let source = if installed {
                painter.dim(agent["sourceLabel"].as_str().unwrap_or(""))
            } else {
                String::new()
            };
            println!(
                "  {name_padded} {status}{}{}",
                if version_text.is_empty() {
                    String::new()
                } else {
                    format!("  {version_text}")
                },
                if update.is_empty() && source.is_empty() {
                    String::new()
                } else if update.is_empty() {
                    format!("  {source}")
                } else if source.is_empty() {
                    format!("  {update}")
                } else {
                    format!("  {update}  {source}")
                }
            );
        }
    }
    println!();
}

fn render_info(result: &CommandResult, painter: &Painter) {
    if let Some(error) = &result.error {
        println!("{}", painter.red(&error.message));
        return;
    }
    let Some(data) = &result.data else {
        return;
    };

    println!(
        "{}\n",
        painter.bold(data["agent"]["displayName"].as_str().unwrap_or("Agent"))
    );
    println!(
        "  Name:         {}",
        data["agent"]["name"].as_str().unwrap_or("-")
    );
    println!(
        "  Aliases:      {}",
        data["agent"]["aliases"]
            .as_array()
            .map(|aliases| {
                aliases
                    .iter()
                    .filter_map(|alias| alias.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .filter(|aliases| !aliases.is_empty())
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "  Package:      {}",
        data["agent"]["packageName"].as_str().unwrap_or("-")
    );
    println!(
        "  Binary:       {}",
        data["agent"]["binaryName"].as_str().unwrap_or("-")
    );
    println!(
        "  Update:       {}",
        data["agent"]["selfUpdateCommands"]
            .as_array()
            .map(|commands| {
                commands
                    .iter()
                    .filter_map(|command| command.as_str())
                    .collect::<Vec<_>>()
                    .join(" || ")
            })
            .filter(|commands| !commands.is_empty())
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "  Installed:    {}",
        if data["inspection"]["installed"].as_bool().unwrap_or(false) {
            painter.green("Yes")
        } else {
            painter.red("No")
        }
    );
    if let Some(source) = data["inspection"]["sourceLabel"].as_str() {
        println!("  Source:       {source}");
    }
    if data["inspection"]["installed"].as_bool().unwrap_or(false)
        && let Some(lifecycle) = data["inspection"]["lifecycle"].as_str()
    {
        println!("  Lifecycle:    {lifecycle}");
    }
    if let Some(version) = data["inspection"]["installedVersion"].as_str() {
        println!("  Version:      {version}");
    }
    if let Some(latest) = data["inspection"]["latestVersion"].as_str() {
        println!("  Latest:       {latest}");
    }
    if let Some(path) = data["inspection"]["binaryPath"].as_str() {
        println!("  Path:         {path}");
    }
    println!("\n  {}:", painter.bold("Install Methods"));
    if let Some(methods) = data["agent"]["installMethods"].as_array() {
        for method in methods {
            println!(
                "    [{}] {}",
                painter.green(&human_install_method_label(method)),
                method["command"].as_str().unwrap_or("")
            );
        }
    }
    println!();
}

#[allow(clippy::too_many_lines)]
fn render_inspect(result: &CommandResult, painter: &Painter) {
    if let Some(error) = &result.error {
        println!("{}", painter.red(&error.message));
        return;
    }
    let Some(data) = &result.data else {
        return;
    };

    println!(
        "{}\n",
        painter.bold(data["agent"]["displayName"].as_str().unwrap_or("Agent"))
    );
    println!(
        "  Name:         {}",
        data["agent"]["name"].as_str().unwrap_or("-")
    );
    println!(
        "  Aliases:      {}",
        data["agent"]["aliases"]
            .as_array()
            .map(|aliases| {
                aliases
                    .iter()
                    .filter_map(|alias| alias.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .filter(|aliases| !aliases.is_empty())
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "  Package:      {}",
        data["agent"]["packageName"].as_str().unwrap_or("-")
    );
    println!(
        "  Binary:       {}",
        data["agent"]["binaryName"].as_str().unwrap_or("-")
    );
    println!(
        "  Installed:    {}",
        if data["inspection"]["installed"].as_bool().unwrap_or(false) {
            painter.green("Yes")
        } else {
            painter.red("No")
        }
    );
    println!(
        "  Update Mode:  {}",
        data["inspection"]["updateLabel"].as_str().unwrap_or("-")
    );
    println!(
        "  Self Update:  {}",
        data["agent"]["selfUpdateCommands"]
            .as_array()
            .map(|commands| {
                commands
                    .iter()
                    .filter_map(|command| command.as_str())
                    .collect::<Vec<_>>()
                    .join(" || ")
            })
            .filter(|commands| !commands.is_empty())
            .unwrap_or_else(|| "-".to_string())
    );
    if let Some(source) = data["inspection"]["sourceLabel"].as_str() {
        println!("  Source:       {source}");
    }
    if let Some(version) = data["inspection"]["installedVersion"].as_str() {
        println!("  Version:      {version}");
    }
    if let Some(latest) = data["inspection"]["latestVersion"].as_str() {
        println!("  Latest:       {latest}");
    }
    if let Some(path) = data["inspection"]["binaryPath"].as_str() {
        println!("  Path:         {path}");
    }
    println!("\n  {}:", painter.bold("Capabilities"));
    println!(
        "    auto-install:   {}",
        yes_no(data["capabilities"]["canAutoInstall"].as_bool(), painter)
    );
    println!(
        "    self-update:    {}",
        yes_no(data["capabilities"]["canSelfUpdate"].as_bool(), painter)
    );
    println!(
        "    auto-uninstall: {}",
        yes_no(data["capabilities"]["canAutoUninstall"].as_bool(), painter)
    );
    println!(
        "    runnable:       {}",
        yes_no(data["capabilities"]["canRun"].as_bool(), painter)
    );
    println!("\n  {}:", painter.bold("Install Methods"));
    if let Some(methods) = data["agent"]["installMethods"].as_array() {
        for method in methods {
            println!(
                "    [{}] {}",
                painter.green(&human_install_method_label(method)),
                method["command"].as_str().unwrap_or("")
            );
        }
    }
    println!();
}

fn render_resolve(result: &CommandResult, painter: &Painter) {
    let Some(data) = &result.data else {
        if let Some(error) = &result.error {
            println!("{}", painter.red(&error.message));
        }
        return;
    };

    if let Some(error) = &result.error {
        println!("{}", painter.red(&error.message));
        if let Some(guidance) = data["resolution"]["installGuidance"].as_object() {
            if let Some(ensure) = guidance["suggestedEnsureCommand"].as_str() {
                println!("{}", painter.dim(&format!("Try: {ensure}")));
            }
            if let Some(methods) = guidance["installMethods"].as_array() {
                for method in methods {
                    let label = method["label"].as_str().unwrap_or("unknown");
                    let command = method["command"].as_str().unwrap_or("");
                    println!("{}", painter.dim(&format!("Install: [{label}] {command}")));
                }
            }
        }
        return;
    }
    if !data["resolution"]["installed"].as_bool().unwrap_or(false) {
        if let Some(ensure) =
            data["resolution"]["installGuidance"]["suggestedEnsureCommand"].as_str()
        {
            println!("{}", painter.dim(&format!("Try: {ensure}")));
        }
        return;
    }

    println!(
        "{}\n",
        painter.bold(data["agent"]["displayName"].as_str().unwrap_or("Agent"))
    );
    println!(
        "  Name:          {}",
        data["agent"]["name"].as_str().unwrap_or("-")
    );
    println!(
        "  Binary:        {}",
        data["agent"]["binaryName"].as_str().unwrap_or("-")
    );
    if let Some(path) = data["resolution"]["binaryPath"].as_str() {
        println!("  Path:          {path}");
    }
    if let Some(source) = data["resolution"]["sourceLabel"].as_str() {
        println!("  Source:        {source}");
    }
    if let Some(lifecycle) = data["resolution"]["lifecycle"].as_str() {
        println!("  Lifecycle:     {lifecycle}");
    }
    if let Some(install_type) = data["resolution"]["installSource"].as_str() {
        println!("  Install Type:  {install_type}");
    }
    if let Some(version) = data["resolution"]["installedVersion"].as_str() {
        println!("  Version:       {version}");
    }
    if let Some(launch) = data["resolution"]["suggestedLaunchCommand"].as_array() {
        let launch = launch
            .iter()
            .filter_map(|part| part.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        println!("  Launch:        {launch}");
    }
    println!();
}

fn render_upgrade(result: &CommandResult, painter: &Painter) {
    if let Some(error) = &result.error {
        if let Some(data) = &result.data {
            if data["status"].as_str() == Some("check-unavailable") {
                println!("{}", painter.red(&error.message));
                render_upgrade_informational_warnings(&result.warnings, painter);
                return;
            }
            if data["status"].as_str() == Some("manual-required")
                && matches!(error.code, AgxErrorCode::ManualActionRequired)
            {
                println!("{}", painter.red(&error.message));
                if let Some(recovery_hint) = data["recoveryHint"].as_str() {
                    println!("{}", painter.dim(&format!("Next step: {recovery_hint}")));
                }
                return;
            }
            let current = data["currentVersion"].as_str().unwrap_or("unknown");
            let version_hint = data["latestVersion"].as_str().map_or_else(
                || format!(" ({current})"),
                |latest| format!(" ({current} -> {latest})"),
            );
            println!("Upgrading AGX CLI...{version_hint}");
            println!("{}", painter.red("Failed to upgrade AGX CLI."));
            println!("{} {}", painter.red("Reason:"), error.message);
            render_upgrade_failure_warnings(&result.warnings, painter);
        } else {
            println!("{}", painter.red(&error.message));
        }
        return;
    }

    let Some(data) = &result.data else {
        return;
    };
    let status = data["status"].as_str().unwrap_or("unknown");
    match status {
        "update-available" => {
            let current = data["currentVersion"].as_str().unwrap_or("unknown");
            let latest = data["latestVersion"].as_str().unwrap_or("unknown");
            let channel = data["channel"].as_str().unwrap_or("stable");
            let dry_run = result
                .warnings
                .iter()
                .any(|warning| warning.code == "DRY_RUN");
            let prefix = if dry_run { "Dry run: " } else { "" };
            println!(
                "{}",
                painter.yellow(&format!(
                    "{prefix}Update available for AGX CLI: {current} -> {latest} ({channel})."
                ))
            );
            render_upgrade_informational_warnings(&result.warnings, painter);
        }
        "up-to-date" => {
            let current = data["currentVersion"].as_str().unwrap_or("unknown");
            println!(
                "{}",
                painter.green(&format!("AGX is already up to date ({current})."))
            );
            render_upgrade_informational_warnings(&result.warnings, painter);
        }
        "planned" => {
            println!("Planned AGX upgrade:");
            if let Some(command) = data["command"].as_array() {
                let command = command
                    .iter()
                    .filter_map(|part| part.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("{command}");
            }
            render_upgrade_informational_warnings(&result.warnings, painter);
        }
        "updated" => {
            let current = data["currentVersion"].as_str().unwrap_or("unknown");
            let version_hint = data["latestVersion"].as_str().map_or_else(
                || format!(" ({current})"),
                |latest| format!(" ({current} -> {latest})"),
            );
            println!("Upgrading AGX CLI...{version_hint}");
            println!("{}", painter.green("AGX CLI upgraded successfully."));
            render_upgrade_informational_warnings(&result.warnings, painter);
        }
        _ => println!("{data}"),
    }
}

fn render_upgrade_informational_warnings(warnings: &[CommandWarning], painter: &Painter) {
    for warning in warnings {
        if matches!(warning.code.as_str(), "DRY_RUN" | "MANUAL_RECOVERY") {
            continue;
        }
        println!("{}", painter.yellow(&warning.message));
    }
}

fn render_upgrade_failure_warnings(warnings: &[CommandWarning], painter: &Painter) {
    for warning in warnings {
        if warning.code == "MANUAL_RECOVERY" {
            println!(
                "{}",
                painter.dim(
                    &warning
                        .message
                        .replacen("Manual recovery:", "Next step:", 1),
                )
            );
        } else if warning.code != "DRY_RUN" {
            println!("{}", painter.yellow(&warning.message));
        }
    }
}

fn render_update(result: &CommandResult, painter: &Painter) {
    if let Some(error) = &result.error {
        println!("{}", painter.red(&error.message));
    }

    let Some(data) = &result.data else {
        return;
    };

    if let Some(results) = data["results"].as_array() {
        for result in results {
            let name = result["displayName"].as_str().unwrap_or("Unknown");
            let status = result["status"].as_str().unwrap_or("unknown");
            match status {
                "up-to-date" => {
                    let version = result["installedVersion"].as_str().unwrap_or("unknown");
                    println!(
                        "{}",
                        painter.green(&format!("{name} is up to date ({version})"))
                    );
                }
                "updated" => {
                    let strategy = result["strategy"].as_str().unwrap_or("update");
                    let version_text = match (
                        result["installedVersion"].as_str(),
                        result["latestVersion"].as_str(),
                    ) {
                        (Some(installed), Some(latest)) => format!(" ({installed} -> {latest})"),
                        _ => String::new(),
                    };
                    println!("Updating {name} via {strategy}...{version_text}");
                    println!(
                        "{}",
                        painter.green(&format!("{name} updated successfully!"))
                    );
                }
                "planned" => println!(
                    "Dry run: would update {name}. {}",
                    result["message"].as_str().unwrap_or("")
                ),
                "manual-required" => {
                    println!(
                        "{}",
                        painter.yellow(&format!("{name}: manual action required."))
                    );
                    if let Some(message) = result["message"].as_str() {
                        println!("{message}");
                    }
                    if let Some(hint) = result["hint"].as_str() {
                        println!("{}", painter.dim(&format!("Next step: {hint}")));
                    }
                }
                "failed" => {
                    let strategy = result["strategy"].as_str().unwrap_or("update");
                    let version_text = match (
                        result["installedVersion"].as_str(),
                        result["latestVersion"].as_str(),
                    ) {
                        (Some(installed), Some(latest)) => format!(" ({installed} -> {latest})"),
                        _ => String::new(),
                    };
                    println!("Updating {name} via {strategy}...{version_text}");
                    println!("{}", painter.red(&format!("Failed to update {name}.")));
                    if let Some(hint) = result["hint"].as_str() {
                        println!("{}", painter.dim(&format!("Next step: {hint}")));
                    }
                }
                "locked" => println!(
                    "{}",
                    result["message"]
                        .as_str()
                        .unwrap_or("Another AGX process is already updating the requested agent.")
                ),
                _ => println!("{result}"),
            }
        }

        if results.len() > 1 {
            render_update_summary(results, painter);
        }
    }
}

fn render_install(result: &CommandResult, painter: &Painter) {
    if let Some(error) = &result.error
        && result
            .data
            .as_ref()
            .and_then(|data| data.get("scope"))
            .and_then(serde_json::Value::as_str)
            != Some("batch")
    {
        eprintln!("{}", painter.red(&error.message));
        return;
    }

    let Some(data) = &result.data else {
        return;
    };

    if data["scope"].as_str() == Some("batch") {
        render_install_batch(data, painter);
        return;
    }

    let display_name = data["agent"]["displayName"].as_str().unwrap_or("Agent");
    if result
        .warnings
        .iter()
        .any(|warning| warning.code == "DRY_RUN")
    {
        println!("Dry run: would install {display_name}.");
        return;
    }

    if let Some(warning) = result.warnings.iter().find(|warning| {
        matches!(
            warning.code.as_str(),
            "ALREADY_INSTALLED" | "TRACKED_EXISTING_INSTALL" | "UNTRACKED_EXISTING_INSTALL"
        )
    }) {
        println!("{}", warning.message);
        return;
    }

    if let Some(message) = data["message"].as_str() {
        println!("{message}");
        return;
    }

    println!("Installing {display_name}...");
    println!(
        "{}",
        painter.green(&format!("{display_name} installed successfully!"))
    );
}

fn render_ensure(result: &CommandResult, painter: &Painter) {
    if let Some(error) = &result.error {
        eprintln!("{}", painter.red(&error.message));
        return;
    }

    let Some(data) = &result.data else {
        return;
    };

    let display_name = data["agent"]["displayName"].as_str().unwrap_or("Agent");
    if result
        .warnings
        .iter()
        .any(|warning| warning.code == "DRY_RUN")
    {
        println!("Dry run: would ensure {display_name}.");
        return;
    }

    if let Some(warning) = result.warnings.iter().find(|warning| {
        matches!(
            warning.code.as_str(),
            "ALREADY_INSTALLED" | "TRACKED_EXISTING_INSTALL" | "UNTRACKED_EXISTING_INSTALL"
        )
    }) {
        println!("{}", warning.message);
        return;
    }

    if let Some(message) = data["message"].as_str() {
        println!("{message}");
        return;
    }

    println!("Installing {display_name}...");
    println!(
        "{}",
        painter.green(&format!("{display_name} is now installed."))
    );
}

fn render_uninstall(result: &CommandResult, painter: &Painter) {
    if let Some(error) = &result.error {
        eprintln!("{}", painter.red(&error.message));
        return;
    }

    let Some(data) = &result.data else {
        return;
    };

    let display_name = data["agent"]["displayName"].as_str().unwrap_or("Agent");
    if result
        .warnings
        .iter()
        .any(|warning| warning.code == "DRY_RUN")
    {
        println!("Dry run: would uninstall {display_name}.");
        return;
    }

    println!("Uninstalling {display_name}...");
    println!(
        "{}",
        painter.green(&format!("{display_name} uninstalled successfully!"))
    );
}

fn render_install_batch(data: &Value, painter: &Painter) {
    if let Some(results) = data["results"].as_array() {
        for item in results {
            let display_name = item["agent"]["displayName"].as_str().unwrap_or("Agent");
            match item["status"].as_str().unwrap_or("unknown") {
                "installed" => {
                    println!("Installing {display_name}...");
                    println!(
                        "{}",
                        painter.green(&format!("{display_name} installed successfully!"))
                    );
                }
                "tracked-existing-install" | "already-installed" | "untracked-existing-install" => {
                    let message = first_batch_warning_message(item)
                        .or_else(|| item["error"]["message"].as_str())
                        .unwrap_or("Install state unchanged.");
                    println!("{message}");
                }
                "planned" => {
                    let message = first_batch_warning_message(item)
                        .unwrap_or("Dry run: would install the requested agent.");
                    println!("{message}");
                }
                "locked" | "failed" => {
                    let message = item["error"]["message"]
                        .as_str()
                        .unwrap_or("Failed to install requested agent.");
                    eprintln!("{}", painter.red(message));
                }
                _ => println!("{item}"),
            }
        }
    }

    let mut parts = Vec::new();
    for (label, key) in [
        ("installed", "installed"),
        ("already installed", "alreadyInstalled"),
        ("tracked existing", "trackedExistingInstall"),
        ("untracked existing", "untrackedExistingInstall"),
        ("failed", "failed"),
        ("locked", "locked"),
        ("planned", "planned"),
    ] {
        let count = data["summary"][key].as_u64().unwrap_or(0);
        if count > 0 {
            parts.push(format!("{label} {count}"));
        }
    }
    if !parts.is_empty() {
        println!("Summary: {}", parts.join(", "));
    }
}

fn first_batch_warning_message(item: &Value) -> Option<&str> {
    item["warnings"]
        .as_array()
        .and_then(|warnings| warnings.first())
        .and_then(|warning| warning["message"].as_str())
}

fn render_update_summary(results: &[Value], painter: &Painter) {
    let mut updated = 0;
    let mut up_to_date = 0;
    let mut manual = 0;
    let mut failed = 0;
    let mut locked = 0;
    let mut planned = 0;

    for result in results {
        match result["status"].as_str().unwrap_or_default() {
            "updated" => updated += 1,
            "up-to-date" => up_to_date += 1,
            "manual-required" => manual += 1,
            "failed" => failed += 1,
            "locked" => locked += 1,
            "planned" => planned += 1,
            _ => {}
        }
    }

    let mut parts = Vec::new();
    if updated > 0 {
        parts.push(format!("updated {updated}"));
    }
    if up_to_date > 0 {
        parts.push(format!("up to date {up_to_date}"));
    }
    if manual > 0 {
        parts.push(format!("manual {manual}"));
    }
    if failed > 0 {
        parts.push(format!("failed {failed}"));
    }
    if locked > 0 {
        parts.push(format!("locked {locked}"));
    }
    if planned > 0 {
        parts.push(format!("planned {planned}"));
    }

    if !parts.is_empty() {
        println!(
            "{}",
            painter.bold(&format!("Summary: {}", parts.join(", ")))
        );
    }
}

fn render_exec(result: &CommandResult, painter: &Painter) {
    if let Some(error) = &result.error {
        eprintln!("{}", painter.red(&error.message));
        if let Some(data) = &result.data {
            if let Some(ensure) =
                data["execution"]["installGuidance"]["suggestedEnsureCommand"].as_str()
            {
                eprintln!("{}", painter.dim(&format!("Try: {ensure}")));
            }
            if let Some(exec) =
                data["execution"]["installGuidance"]["suggestedExecCommand"].as_str()
            {
                eprintln!("{}", painter.dim(&format!("Or:  {exec}")));
            }
        }
        return;
    }

    if let Some(data) = &result.data {
        let launched = data["execution"]["launched"].as_bool().unwrap_or(false);
        let installed = data["execution"]["installed"].as_bool().unwrap_or(false);
        if !launched {
            let binary_name = data["agent"]["binaryName"]
                .as_str()
                .or_else(|| data["agent"]["name"].as_str())
                .unwrap_or("agent");
            let args = data["execution"]["args"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            if installed {
                if args.is_empty() {
                    println!("Dry run: would run {binary_name}");
                } else {
                    println!("Dry run: would run {binary_name} {args}");
                }
            } else {
                println!(
                    "Dry run: would install {}",
                    data["agent"]["displayName"].as_str().unwrap_or("agent")
                );
            }
        }
    }
}

fn yes_no(value: Option<bool>, painter: &Painter) -> String {
    if value.unwrap_or(false) {
        painter.yes()
    } else {
        painter.no()
    }
}

fn human_install_method_label(method: &Value) -> String {
    match method["type"].as_str() {
        Some("bun") => "bun".to_string(),
        Some("npm") => "npm".to_string(),
        Some(other) => method["label"]
            .as_str()
            .map_or_else(|| other.to_string(), ToString::to_string),
        None => method["label"]
            .as_str()
            .map_or_else(|| "unknown".to_string(), ToString::to_string),
    }
}
