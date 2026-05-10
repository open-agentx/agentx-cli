use std::io::IsTerminal;
use std::sync::{Arc, Mutex};

use crate::cli::{Cli, ColorModeArg, LogLevelArg, OutputModeArg};
use crate::errors::{AgxError, AgxErrorCode};

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputMode {
    Human,
    Json,
    Ndjson,
}

impl OutputMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Json => "json",
            Self::Ndjson => "ndjson",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CacheMode {
    Default,
    NoCache,
    Refresh,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FreshnessSource {
    Cache,
    Network,
}

impl FreshnessSource {
    pub const fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Network, Self::Network) => Self::Network,
            _ => Self::Cache,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CliFreshness {
    pub fetched_at: String,
    pub source: FreshnessSource,
    pub stale_after: String,
}

#[derive(Debug, Clone, Copy)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Silent,
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug)]
#[allow(dead_code)]
#[allow(clippy::struct_excessive_bools)]
pub struct CliContext {
    pub assume_yes: bool,
    pub cache_mode: CacheMode,
    pub color_mode: ColorMode,
    pub dry_run: bool,
    pub freshness: Arc<Mutex<Option<CliFreshness>>>,
    pub idempotency_key: Option<String>,
    pub interactive: bool,
    pub log_level: LogLevel,
    pub output_mode: OutputMode,
    pub quiet: bool,
    pub run_id: String,
    pub timeout_ms: Option<u64>,
}

impl TryFrom<&Cli> for CliContext {
    type Error = AgxError;

    fn try_from(cli: &Cli) -> Result<Self, Self::Error> {
        if cli.refresh && cli.no_cache {
            return Err(AgxError::new(
                AgxErrorCode::InvalidArgument,
                "Cannot combine --refresh with --no-cache.",
            ));
        }

        if cli.json && cli.output.is_some() {
            return Err(AgxError::new(
                AgxErrorCode::InvalidArgument,
                "Cannot combine --json with --output.",
            ));
        }

        let output_mode = if cli.json {
            OutputMode::Json
        } else {
            match cli.output {
                Some(OutputModeArg::Human) | None => OutputMode::Human,
                Some(OutputModeArg::Json) => OutputMode::Json,
                Some(OutputModeArg::Ndjson) => OutputMode::Ndjson,
            }
        };

        Ok(Self {
            assume_yes: cli.yes,
            cache_mode: if cli.no_cache {
                CacheMode::NoCache
            } else if cli.refresh {
                CacheMode::Refresh
            } else {
                CacheMode::Default
            },
            color_mode: match cli.color.unwrap_or(ColorModeArg::Auto) {
                ColorModeArg::Auto => ColorMode::Auto,
                ColorModeArg::Always => ColorMode::Always,
                ColorModeArg::Never => ColorMode::Never,
            },
            dry_run: cli.dry_run,
            freshness: Arc::new(Mutex::new(None)),
            idempotency_key: cli.idempotency_key.clone(),
            interactive: !cli.non_interactive,
            log_level: match cli.log_level.unwrap_or(LogLevelArg::Info) {
                LogLevelArg::Silent => LogLevel::Silent,
                LogLevelArg::Error => LogLevel::Error,
                LogLevelArg::Warn => LogLevel::Warn,
                LogLevelArg::Info => LogLevel::Info,
                LogLevelArg::Debug => LogLevel::Debug,
            },
            output_mode,
            quiet: cli.quiet,
            run_id: cli
                .run_id
                .clone()
                .or_else(|| std::env::var("AGX_RUN_ID").ok())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            timeout_ms: cli
                .timeout
                .as_deref()
                .map(parse_duration_to_ms)
                .transpose()?,
        })
    }
}

impl ColorMode {
    pub fn enabled(self) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,
            Self::Auto => std::io::stdout().is_terminal(),
        }
    }
}

fn parse_duration_to_ms(input: &str) -> Result<u64, AgxError> {
    let (number, multiplier) = if let Some(value) = input.strip_suffix("ms") {
        (value, 1)
    } else if let Some(value) = input.strip_suffix('s') {
        (value, 1_000)
    } else if let Some(value) = input.strip_suffix('m') {
        (value, 60_000)
    } else {
        (input, 1)
    };

    let parsed = number.parse::<u64>().map_err(|_| {
        AgxError::new(
            AgxErrorCode::InvalidArgument,
            format!("Invalid timeout value: {input}"),
        )
    })?;

    parsed.checked_mul(multiplier).ok_or_else(|| {
        AgxError::new(
            AgxErrorCode::InvalidArgument,
            format!("Invalid timeout value: {input}"),
        )
    })
}

pub fn record_freshness(context: &CliContext, freshness: CliFreshness) {
    let mut slot = context
        .freshness
        .lock()
        .expect("freshness mutex should not be poisoned");
    if let Some(existing) = slot.as_mut() {
        if freshness.fetched_at < existing.fetched_at {
            existing.fetched_at.clone_from(&freshness.fetched_at);
        }
        existing.source = existing.source.merge(freshness.source);
        if freshness.stale_after < existing.stale_after {
            existing.stale_after = freshness.stale_after;
        }
    } else {
        *slot = Some(freshness);
    }
}
