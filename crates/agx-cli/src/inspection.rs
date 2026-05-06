use std::path::{Path, PathBuf};
use std::process::Command;

use crate::agents::AgentDefinition;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInspection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_path: Option<String>,
    pub installed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    pub lifecycle: &'static str,
    pub source_label: &'static str,
    pub update_label: &'static str,
}

pub fn inspect_agent(agent: AgentDefinition) -> AgentInspection {
    let binary_path = find_binary_in_path(agent.binary_name);
    let installed = binary_path.is_some();
    let installed_version = binary_path.as_ref().and_then(get_installed_version);

    AgentInspection {
        binary_path: binary_path.map(|path| path.to_string_lossy().into_owned()),
        installed,
        installed_version,
        latest_version: None,
        lifecycle: "unmanaged",
        source_label: "untracked",
        update_label: "manual",
    }
}

pub fn find_binary_in_path(binary_name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|directory| {
        let candidate = directory.join(binary_name);
        if candidate.is_file() {
            return Some(candidate);
        }

        if cfg!(windows) {
            let exe_candidate = directory.join(format!("{binary_name}.exe"));
            if exe_candidate.is_file() {
                return Some(exe_candidate);
            }
        }

        None
    })
}

fn get_installed_version(binary_path: &PathBuf) -> Option<String> {
    let output = Command::new(binary_path)
        .args(version_probe_args(binary_path))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_version(&stdout)
}

fn version_probe_args(binary_path: &Path) -> &'static [&'static str] {
    let binary_name = binary_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    match binary_name {
        "amp" | "devin" => &["version"],
        _ => &["--version"],
    }
}

fn parse_version(stdout: &str) -> Option<String> {
    stdout
        .split_whitespace()
        .find(|part| {
            part.chars()
                .next()
                .is_some_and(|char| char.is_ascii_digit())
        })
        .map(|part| {
            part.trim_matches(|char: char| {
                !(char.is_ascii_alphanumeric() || matches!(char, '.' | '-' | '+'))
            })
            .to_string()
        })
}
