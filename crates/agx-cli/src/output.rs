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
    if let Some(error) = &result.error {
        eprintln!("{}", error.message);
        return;
    }

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
        _ => {
            if let Some(data) = &result.data {
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
