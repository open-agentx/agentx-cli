use serde::Serialize;
use serde_json::Value;

use crate::context::{CliContext, OutputMode};
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: AgxErrorCode,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandMeta {
    pub mode: &'static str,
    pub run_id: String,
    pub schema_version: &'static str,
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
        let exit_code = error.exit_code();
        Self {
            action: action.into(),
            data: None,
            error: Some(CommandError {
                code: error.code,
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
        let exit_code = error.exit_code();
        Self {
            action: action.into(),
            data: Some(serde_json::to_value(data).expect("command data must serialize")),
            error: Some(CommandError {
                code: error.code,
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
            render_human(result);
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

fn create_meta(context: &CliContext) -> CommandMeta {
    CommandMeta {
        mode: context.output_mode.as_str(),
        run_id: context.run_id.clone(),
        schema_version: SCHEMA_VERSION,
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
        meta: CommandMeta {
            mode: "ndjson",
            run_id: context.run_id.clone(),
            schema_version: SCHEMA_VERSION,
            timestamp: current_timestamp(),
            version: env!("CARGO_PKG_VERSION"),
        },
        kind: "result",
    }
}

fn render_human(result: &CommandResult) {
    match result.action.as_str() {
        "commands" => {
            println!("AGX Commands\n\nRun `agx commands --json` for the stable command catalog.");
        }
        "schema" => {
            println!("AGX Schemas\n\nRun `agx schema --json` for structured output schemas.");
        }
        "capabilities" => println!(
            "AGX Capabilities\n\nRun `agx capabilities --json` for environment capabilities."
        ),
        "doctor" => {
            if let Some(data) = &result.data {
                render_doctor(data);
            }
        }
        "list" => render_list(result),
        "info" => render_info(result),
        "inspect" => render_inspect(result),
        "resolve" => render_resolve(result),
        "upgrade" => render_upgrade(result),
        _ => {
            if let Some(error) = &result.error {
                eprintln!("{}", error.message);
            } else if let Some(data) = &result.data {
                println!("{data}");
            }
        }
    }
}

fn current_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    format!("{seconds}")
}

#[allow(clippy::too_many_lines)]
fn render_doctor(data: &Value) {
    println!("AGX Environment Check\n");
    if let Some(summary) = data["summary"].as_str() {
        println!("{summary}\n");
    }
    println!("Managed Installers:");
    println!(
        "  Bun:   {}",
        if data["installers"]["bun"].as_bool().unwrap_or(false) {
            "available"
        } else {
            "not found"
        }
    );
    println!(
        "  npm:   {}",
        if data["installers"]["npm"].as_bool().unwrap_or(false) {
            "available"
        } else {
            "not found"
        }
    );
    println!(
        "  brew:  {}",
        if data["installers"]["brew"].as_bool().unwrap_or(false) {
            "available"
        } else {
            "not found"
        }
    );
    println!(
        "  winget:{}",
        if data["installers"]["winget"].as_bool().unwrap_or(false) {
            "available"
        } else {
            "not found"
        }
    );

    println!("\nAGX CLI:");
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
            "supported"
        } else {
            "unsupported"
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

    println!("\nInstalled Agents:");
    if data["agents"]
        .as_array()
        .is_some_and(std::vec::Vec::is_empty)
    {
        println!("  No agents installed");
    } else if let Some(agents) = data["agents"].as_array() {
        for agent in agents {
            println!(
                "  {}: {} [{}; {}]{}",
                agent["displayName"].as_str().unwrap_or("unknown"),
                agent["installedVersion"].as_str().unwrap_or("unknown"),
                agent["lifecycle"].as_str().unwrap_or("unknown"),
                agent["sourceLabel"].as_str().unwrap_or("unknown"),
                if agent["outdated"].as_bool().unwrap_or(false) {
                    format!(
                        " (update available: {})",
                        agent["latestVersion"].as_str().unwrap_or("unknown")
                    )
                } else {
                    String::new()
                }
            );
        }
    }

    println!("\nIssues:");
    if data["issues"]
        .as_array()
        .is_some_and(std::vec::Vec::is_empty)
    {
        println!("  No issues found.");
    } else if let Some(issues) = data["issues"].as_array() {
        for issue in issues {
            println!(
                "  - {}",
                issue["message"].as_str().unwrap_or("unknown issue")
            );
            if let Some(commands) = issue["suggestedCommands"].as_array()
                && !commands.is_empty()
            {
                let next: Vec<_> = commands
                    .iter()
                    .filter_map(|command| command.as_str())
                    .collect();
                println!("    Next: {}", next.join(" | "));
            }
        }
    }

    println!();
}

fn render_list(result: &CommandResult) {
    println!("AI Agents:\n");
    if let Some(agents) = result
        .data
        .as_ref()
        .and_then(|data| data["agents"].as_array())
    {
        for agent in agents {
            let status = if agent["installed"].as_bool().unwrap_or(false) {
                "installed"
            } else {
                "not installed"
            };
            let version = agent["installedVersion"].as_str().unwrap_or("");
            let update = agent["updateLabel"].as_str().unwrap_or("");
            let source = agent["sourceLabel"].as_str().unwrap_or("");
            println!(
                "  {}: {}{}{}{}",
                agent["displayName"].as_str().unwrap_or("unknown"),
                status,
                if version.is_empty() {
                    String::new()
                } else {
                    format!(" ({version})")
                },
                if update.is_empty() {
                    String::new()
                } else {
                    format!(" [{update}]")
                },
                if source.is_empty() {
                    String::new()
                } else {
                    format!(" {source}")
                }
            );
        }
    }
    println!();
}

fn render_info(result: &CommandResult) {
    if let Some(error) = &result.error {
        println!("{}", error.message);
        return;
    }
    let Some(data) = &result.data else {
        return;
    };

    println!(
        "{}\n",
        data["agent"]["displayName"].as_str().unwrap_or("Agent")
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
            "Yes"
        } else {
            "No"
        }
    );
    if let Some(source) = data["inspection"]["sourceLabel"].as_str() {
        println!("  Source:       {source}");
    }
    if let Some(lifecycle) = data["inspection"]["lifecycle"].as_str() {
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
    println!("\n  Install Methods:");
    if let Some(methods) = data["agent"]["installMethods"].as_array() {
        for method in methods {
            println!(
                "    [{}] {}",
                method["label"].as_str().unwrap_or("unknown"),
                method["command"].as_str().unwrap_or("")
            );
        }
    }
    println!();
}

fn render_inspect(result: &CommandResult) {
    if let Some(error) = &result.error {
        println!("{}", error.message);
        return;
    }
    let Some(data) = &result.data else {
        return;
    };

    println!(
        "{}\n",
        data["agent"]["displayName"].as_str().unwrap_or("Agent")
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
            "Yes"
        } else {
            "No"
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
    if let Some(path) = data["inspection"]["binaryPath"].as_str() {
        println!("  Path:         {path}");
    }
    println!("\n  Capabilities:");
    println!(
        "    auto-install:   {}",
        yes_no(data["capabilities"]["canAutoInstall"].as_bool())
    );
    println!(
        "    self-update:    {}",
        yes_no(data["capabilities"]["canSelfUpdate"].as_bool())
    );
    println!(
        "    auto-uninstall: {}",
        yes_no(data["capabilities"]["canAutoUninstall"].as_bool())
    );
    println!(
        "    runnable:       {}",
        yes_no(data["capabilities"]["canRun"].as_bool())
    );
    println!("\n  Install Methods:");
    if let Some(methods) = data["agent"]["installMethods"].as_array() {
        for method in methods {
            println!(
                "    [{}] {}",
                method["label"].as_str().unwrap_or("unknown"),
                method["command"].as_str().unwrap_or("")
            );
        }
    }
    println!();
}

fn render_resolve(result: &CommandResult) {
    let Some(data) = &result.data else {
        if let Some(error) = &result.error {
            println!("{}", error.message);
        }
        return;
    };

    if let Some(error) = &result.error {
        println!("{}", error.message);
    }
    if !data["resolution"]["installed"].as_bool().unwrap_or(false) {
        if let Some(ensure) =
            data["resolution"]["installGuidance"]["suggestedEnsureCommand"].as_str()
        {
            println!("{ensure}");
        }
        return;
    }

    println!(
        "{}\n",
        data["agent"]["displayName"].as_str().unwrap_or("Agent")
    );
    if let Some(path) = data["resolution"]["binaryPath"].as_str() {
        println!("  Path:          {path}");
    }
    if let Some(install_type) = data["resolution"]["installSource"].as_str() {
        println!("  Install Type:  {install_type}");
    }
    if let Some(source) = data["resolution"]["sourceLabel"].as_str() {
        println!("  Source:        {source}");
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

fn render_upgrade(result: &CommandResult) {
    if let Some(error) = &result.error {
        println!("{}", error.message);
        if let Some(data) = &result.data
            && let Some(message) = data["message"].as_str()
        {
            println!("{message}");
        }
        return;
    }

    let Some(data) = &result.data else {
        return;
    };
    let status = data["status"].as_str().unwrap_or("unknown");
    match status {
        "update-available" => {
            let latest = data["latestVersion"].as_str().unwrap_or("unknown");
            let channel = data["channel"].as_str().unwrap_or("stable");
            println!("Update available: {latest} ({channel})");
        }
        "up-to-date" => {
            println!("AGX is already up to date.");
            if let Some(message) = data["message"].as_str() {
                println!("{message}");
            }
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
        }
        "upgraded" => println!("AGX upgraded successfully."),
        _ => println!("{data}"),
    }
}

fn yes_no(value: Option<bool>) -> &'static str {
    if value.unwrap_or(false) { "yes" } else { "no" }
}
