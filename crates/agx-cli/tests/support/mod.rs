#![allow(dead_code)]

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct TestWorkspace {
    root: PathBuf,
}

impl TestWorkspace {
    pub fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0_u128, |duration| duration.as_nanos());
        let root = std::env::temp_dir().join(format!("agx-tests-{unique}"));
        fs::create_dir_all(&root).expect("failed to create test workspace");
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config_dir(&self) -> PathBuf {
        self.root.join(".quantex")
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir().join("config.json")
    }

    pub fn state_file(&self) -> PathBuf {
        self.config_dir().join("state.json")
    }

    pub fn cache_file(&self) -> PathBuf {
        self.config_dir().join("cache").join("versions.json")
    }

    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }

    pub fn bun_bin_dir(&self) -> PathBuf {
        self.root.join(".bun").join("bin")
    }

    pub fn npm_bin_dir(&self) -> PathBuf {
        self.root.join("node_modules").join(".bin")
    }

    pub fn write_config_bytes(&self, contents: &[u8]) {
        fs::create_dir_all(self.config_dir()).expect("failed to create config dir");
        fs::write(self.config_file(), contents).expect("failed to write config file");
    }

    pub fn write_state_bytes(&self, contents: &[u8]) {
        fs::create_dir_all(self.config_dir()).expect("failed to create config dir");
        fs::write(self.state_file(), contents).expect("failed to write state file");
    }

    pub fn install_fake_agent_binary(&self, binary_name: &str) -> PathBuf {
        let source = PathBuf::from(env!("CARGO_BIN_EXE_agx"));
        let extension = if cfg!(windows) { ".exe" } else { "" };
        let destination = self.bin_dir().join(format!("{binary_name}{extension}"));
        fs::create_dir_all(self.bin_dir()).expect("failed to create bin dir");
        fs::copy(source, &destination).expect("failed to copy test binary");
        destination
    }

    pub fn install_fake_bun_agent_binary(&self, binary_name: &str) -> PathBuf {
        Self::install_fake_agent_binary_in_dir(&self.bun_bin_dir(), binary_name)
    }

    pub fn install_fake_npm_agent_binary(&self, binary_name: &str) -> PathBuf {
        Self::install_fake_agent_binary_in_dir(&self.npm_bin_dir(), binary_name)
    }

    pub fn install_fake_self_binary(&self) -> PathBuf {
        Self::install_fake_agent_binary_in_dir(&self.root.join("standalone"), "agx")
    }

    fn install_fake_agent_binary_in_dir(directory: &Path, binary_name: &str) -> PathBuf {
        let source = PathBuf::from(env!("CARGO_BIN_EXE_agx"));
        let extension = if cfg!(windows) { ".exe" } else { "" };
        let destination = directory.join(format!("{binary_name}{extension}"));
        fs::create_dir_all(directory).expect("failed to create bin dir");
        fs::copy(source, &destination).expect("failed to copy test binary");
        destination
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn run_agx(workspace: &TestWorkspace, args: &[&str]) -> Output {
    run_agx_with_env(workspace, args, &[])
}

pub fn run_agx_with_env(workspace: &TestWorkspace, args: &[&str], envs: &[(&str, &str)]) -> Output {
    run_agx_with_io(workspace, args, envs, None)
}

pub fn run_agx_with_stdin(
    workspace: &TestWorkspace,
    args: &[&str],
    envs: &[(&str, &str)],
    stdin: &str,
) -> Output {
    run_agx_with_io(workspace, args, envs, Some(stdin))
}

fn run_agx_with_io(
    workspace: &TestWorkspace,
    args: &[&str],
    envs: &[(&str, &str)],
    stdin: Option<&str>,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_agx"));
    command.args(args);
    command.env("USERPROFILE", workspace.root());
    command.env("HOME", workspace.root());
    command.env("AGX_RUN_ID", "test-run-id");
    command.env("PATH", build_test_path(workspace));
    for (key, value) in envs {
        command.env(key, value);
    }
    if let Some(stdin) = stdin {
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        let mut child = command.spawn().expect("failed to spawn agx");
        if let Some(mut writer) = child.stdin.take() {
            use std::io::Write;
            writer
                .write_all(stdin.as_bytes())
                .expect("failed to write agx stdin");
        }
        child.wait_with_output().expect("failed to wait for agx")
    } else {
        command.output().expect("failed to run agx")
    }
}

pub fn stdout_json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).expect("stdout should be valid json")
}

pub fn stdout_json_lines(output: &Output) -> Vec<serde_json::Value> {
    stdout_text(output)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("stdout line should be valid json"))
        .collect()
}

pub fn stdout_text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf8")
}

pub fn stdout_contains_ansi(output: &Output) -> bool {
    stdout_text(output).contains("\u{1b}[")
}

fn build_test_path(workspace: &TestWorkspace) -> OsString {
    std::env::join_paths([
        workspace.bin_dir(),
        workspace.bun_bin_dir(),
        workspace.npm_bin_dir(),
    ])
    .expect("failed to join PATH")
}
