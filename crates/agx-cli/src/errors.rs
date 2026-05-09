#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(dead_code)]
pub enum AgxErrorCode {
    AgentNotFound,
    AgentNotInstalled,
    Cancelled,
    InstallFailed,
    InteractionRequired,
    InvalidArgument,
    ManualActionRequired,
    NetworkError,
    ResourceLocked,
    Timeout,
    UninstallFailed,
    UpdateFailed,
    UpgradeFailed,
}

impl AgxErrorCode {
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::InvalidArgument => 2,
            Self::AgentNotFound => 3,
            Self::AgentNotInstalled => 4,
            Self::NetworkError => 6,
            Self::InteractionRequired => 7,
            Self::ManualActionRequired => 8,
            Self::ResourceLocked => 9,
            Self::Timeout => 10,
            Self::Cancelled => 11,
            Self::InstallFailed
            | Self::UninstallFailed
            | Self::UpdateFailed
            | Self::UpgradeFailed => 1,
        }
    }
}

#[derive(Debug)]
pub struct AgxError {
    pub code: AgxErrorCode,
    pub message: String,
}

impl AgxError {
    pub fn new(code: AgxErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub const fn exit_code(&self) -> u8 {
        self.code.exit_code()
    }
}
