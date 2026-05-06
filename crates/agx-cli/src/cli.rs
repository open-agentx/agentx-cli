use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[allow(clippy::struct_excessive_bools)]
#[command(
    name = "agx",
    version,
    about = "Rust-native AI agent lifecycle CLI",
    disable_help_subcommand = true
)]
pub struct Cli {
    #[arg(long, global = true, help = "Output structured JSON")]
    pub json: bool,

    #[arg(
        long,
        global = true,
        value_enum,
        help = "Output mode: human, json, or ndjson"
    )]
    pub output: Option<OutputModeArg>,

    #[arg(
        long,
        global = true,
        help = "Disable interactive prompts and confirmations"
    )]
    pub non_interactive: bool,

    #[arg(
        long,
        global = true,
        help = "Automatically accept safe default confirmations"
    )]
    pub yes: bool,

    #[arg(long, global = true, help = "Suppress non-essential human logs")]
    pub quiet: bool,

    #[arg(
        long,
        global = true,
        value_enum,
        help = "Color mode: auto, always, or never"
    )]
    pub color: Option<ColorModeArg>,

    #[arg(
        long,
        global = true,
        value_enum,
        help = "Log level: silent, error, warn, info, or debug"
    )]
    pub log_level: Option<LogLevelArg>,

    #[arg(
        long,
        global = true,
        help = "Show what would happen without making changes"
    )]
    pub dry_run: bool,

    #[arg(
        long,
        global = true,
        help = "Refresh cached version metadata before returning results"
    )]
    pub refresh: bool,

    #[arg(
        long,
        global = true,
        help = "Bypass the local version cache for this command"
    )]
    pub no_cache: bool,

    #[arg(
        long,
        global = true,
        help = "Attach a run id to structured output and logs"
    )]
    pub run_id: Option<String>,

    #[arg(
        long,
        global = true,
        help = "Deduplicate repeated mutating requests by client-supplied key"
    )]
    pub idempotency_key: Option<String>,

    #[arg(
        long,
        global = true,
        help = "Abort a command after the given duration, e.g. 500ms, 30s, 5m"
    )]
    pub timeout: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputModeArg {
    Human,
    Json,
    Ndjson,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ColorModeArg {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LogLevelArg {
    Silent,
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum InstallPolicyArg {
    Never,
    IfMissing,
    Always,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Return environment and surface capabilities")]
    Capabilities,

    #[command(about = "Return the stable command catalog")]
    Commands,

    #[command(about = "Return structured output schemas")]
    Schema {
        #[arg(help = "Optional command name")]
        command: Option<String>,
    },

    #[command(alias = "ls", about = "List supported agents")]
    List,

    #[command(about = "Show agent details")]
    Info {
        #[arg(help = "Agent name or alias")]
        agent: String,
    },

    #[command(alias = "i", about = "Install an agent")]
    Install {
        #[arg(help = "Agent name or alias")]
        agent: String,
    },

    #[command(about = "Ensure an agent is installed")]
    Ensure {
        #[arg(help = "Agent name or alias")]
        agent: String,
    },

    #[command(about = "Execute an agent command")]
    Exec {
        #[arg(help = "Agent name or alias")]
        agent: String,
        #[arg(
            long,
            value_enum,
            default_value = "if-missing",
            help = "Install policy before executing: never, if-missing, or always"
        )]
        install_policy: InstallPolicyArg,
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "Arguments passed to the agent after --"
        )]
        args: Vec<String>,
    },

    #[command(about = "Return structured agent state")]
    Inspect {
        #[arg(help = "Agent name or alias")]
        agent: String,
    },

    #[command(about = "Resolve an agent executable entrypoint")]
    Resolve {
        #[arg(help = "Agent name or alias")]
        agent: String,
    },

    #[command(about = "Read and modify AGX configuration")]
    Config {
        #[arg(help = "Action: get, set, reset")]
        action: Option<String>,
        #[arg(help = "Config key")]
        key: Option<String>,
        #[arg(help = "Config value")]
        value: Option<String>,
    },

    #[command(alias = "rm", about = "Uninstall an agent")]
    Uninstall {
        #[arg(help = "Agent name or alias")]
        agent: String,
    },

    #[command(alias = "u", about = "Update one or all agents")]
    Update {
        #[arg(help = "Agent name or alias")]
        agent: Option<String>,
        #[arg(long, help = "Update all tracked installed agents")]
        all: bool,
    },
}
