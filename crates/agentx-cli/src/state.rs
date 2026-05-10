use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::errors::{AgxError, AgxErrorCode};
use crate::lock::with_resource_lock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledAgentState {
    pub agent_name: String,
    pub install_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_target_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_install_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_notice_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_notice_version: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgxState {
    pub installed_agents: BTreeMap<String, InstalledAgentState>,
    #[serde(rename = "self")]
    #[serde(default)]
    pub self_state: SelfState,
}

pub fn load_state() -> AgxState {
    read_state().unwrap_or_default()
}

#[allow(dead_code)]
pub fn save_state(state: &AgxState) -> Result<(), AgxError> {
    with_resource_lock("state", || write_state(state))
}

#[allow(dead_code)]
pub fn get_installed_agent_state(agent_name: &str) -> Option<InstalledAgentState> {
    load_state().installed_agents.remove(agent_name)
}

#[allow(dead_code)]
pub fn set_installed_agent_state(agent_state: InstalledAgentState) -> Result<(), AgxError> {
    mutate_state(|state| {
        state
            .installed_agents
            .insert(agent_state.agent_name.clone(), agent_state);
    })
}

#[allow(dead_code)]
pub fn remove_installed_agent_state(agent_name: &str) -> Result<(), AgxError> {
    mutate_state(|state| {
        state.installed_agents.remove(agent_name);
    })
}

#[allow(dead_code)]
pub fn set_self_install_source(install_source: impl Into<String>) -> Result<(), AgxError> {
    let install_source = install_source.into();
    mutate_state(|state| {
        state.self_state.install_source = Some(install_source);
    })
}

fn mutate_state(mutator: impl FnOnce(&mut AgxState)) -> Result<(), AgxError> {
    with_resource_lock("state", || {
        let mut state = read_state().unwrap_or_default();
        mutator(&mut state);
        write_state(&state)
    })
}

fn read_state() -> Result<AgxState, AgxError> {
    let path = state_file_path();
    let raw = fs::read_to_string(path).map_err(|error| {
        AgxError::new(
            AgxErrorCode::InvalidArgument,
            format!("Failed to read state: {error}"),
        )
    })?;

    serde_json::from_str(strip_json_bom(&raw)).map_err(|error| {
        AgxError::new(
            AgxErrorCode::InvalidArgument,
            format!("Failed to parse state: {error}"),
        )
    })
}

fn write_state(state: &AgxState) -> Result<(), AgxError> {
    let path = state_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AgxError::new(
                AgxErrorCode::InvalidArgument,
                format!("Failed to create state directory: {error}"),
            )
        })?;
    }

    let json = serde_json::to_string_pretty(state).map_err(|error| {
        AgxError::new(
            AgxErrorCode::InvalidArgument,
            format!("Failed to serialize state: {error}"),
        )
    })?;

    fs::write(path, format!("{json}\n")).map_err(|error| {
        AgxError::new(
            AgxErrorCode::InvalidArgument,
            format!("Failed to write state: {error}"),
        )
    })
}

pub fn state_file_path() -> PathBuf {
    config_dir().join("state.json")
}

fn config_dir() -> PathBuf {
    home_dir().join(".quantex")
}

fn strip_json_bom(raw: &str) -> &str {
    raw.strip_prefix('\u{feff}').unwrap_or(raw)
}

fn home_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}
