mod support;

use std::fs;

use support::{TestWorkspace, run_agx, stdout_json};

#[test]
fn config_without_action_lists_default_values() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "config"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["action"], "list");
    assert_eq!(json["data"]["config"]["defaultPackageManager"], "bun");
    assert_eq!(json["data"]["config"]["networkRetries"], 2);
}

#[test]
fn config_set_persists_compatible_quantex_config() {
    let workspace = TestWorkspace::new();
    let output = run_agx(
        &workspace,
        &["--json", "config", "set", "defaultPackageManager", "npm"],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["action"], "set");
    assert_eq!(json["data"]["value"], "npm");

    let stored = fs::read_to_string(workspace.config_file()).expect("config file should exist");
    assert!(stored.contains("\"defaultPackageManager\": \"npm\""));
}

#[test]
fn config_reads_bom_prefixed_compatible_json() {
    let workspace = TestWorkspace::new();
    workspace.write_config_bytes(b"\xEF\xBB\xBF{\"defaultPackageManager\":\"npm\"}\n");

    let output = run_agx(
        &workspace,
        &["--json", "config", "get", "defaultPackageManager"],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["value"], "npm");
}

#[test]
fn config_get_returns_null_for_missing_key() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "config", "get", "nonexistent"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert!(json["data"]["value"].is_null());
}

#[test]
fn config_get_human_output_prints_not_set_for_missing_key() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["config", "get", "nonexistent"]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout.trim(), "(not set)");
}

#[test]
fn config_list_human_output_prints_pretty_json() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["config"]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Current Configuration:"));
    assert!(stdout.contains("\"defaultPackageManager\": \"bun\""));
    assert!(stdout.contains("\"networkRetries\": 2"));
}

#[test]
fn config_set_normalizes_registry_url() {
    let workspace = TestWorkspace::new();
    let output = run_agx(
        &workspace,
        &[
            "--json",
            "config",
            "set",
            "selfUpdateRegistry",
            "https://registry.npmjs.org/",
        ],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["value"], "https://registry.npmjs.org");

    let stored = fs::read_to_string(workspace.config_file()).expect("config file should exist");
    assert!(stored.contains("\"selfUpdateRegistry\": \"https://registry.npmjs.org\""));
}

#[test]
fn config_set_persists_beta_self_update_channel() {
    let workspace = TestWorkspace::new();
    let output = run_agx(
        &workspace,
        &["--json", "config", "set", "selfUpdateChannel", "beta"],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["value"], "beta");

    let stored = fs::read_to_string(workspace.config_file()).expect("config file should exist");
    assert!(stored.contains("\"selfUpdateChannel\": \"beta\""));
}

#[test]
fn config_set_persists_npm_bun_update_strategy() {
    let workspace = TestWorkspace::new();
    let output = run_agx(
        &workspace,
        &[
            "--json",
            "config",
            "set",
            "npmBunUpdateStrategy",
            "respect-semver",
        ],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["value"], "respect-semver");

    let stored = fs::read_to_string(workspace.config_file()).expect("config file should exist");
    assert!(stored.contains("\"npmBunUpdateStrategy\": \"respect-semver\""));
}

#[test]
fn config_set_persists_numeric_timeout_values() {
    let workspace = TestWorkspace::new();
    let output = run_agx(
        &workspace,
        &["--json", "config", "set", "networkTimeoutMs", "15000"],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["value"], 15000);

    let stored = fs::read_to_string(workspace.config_file()).expect("config file should exist");
    assert!(stored.contains("\"networkTimeoutMs\": 15000"));
}

#[test]
fn config_set_rejects_unknown_keys() {
    let workspace = TestWorkspace::new();
    let output = run_agx(
        &workspace,
        &["--json", "config", "set", "nonexistentKey", "value"],
    );

    assert_eq!(output.status.code(), Some(2));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("Unknown config key")
    );
}

#[test]
fn config_set_rejects_invalid_registry_urls() {
    let workspace = TestWorkspace::new();
    let output = run_agx(
        &workspace,
        &["--json", "config", "set", "selfUpdateRegistry", "npmjs"],
    );

    assert_eq!(output.status.code(), Some(2));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("valid absolute URL")
    );
}

#[test]
fn config_set_rejects_invalid_numeric_values() {
    let workspace = TestWorkspace::new();
    let output = run_agx(
        &workspace,
        &["--json", "config", "set", "versionCacheTtlHours", "0"],
    );

    assert_eq!(output.status.code(), Some(2));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("positive integer")
    );
}

#[test]
fn update_all_dry_run_reads_bom_prefixed_state_file() {
    let workspace = TestWorkspace::new();
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {
    "qoder": {
      "agentName": "qoder",
      "installType": "bun",
      "packageName": "@qoder-ai/qodercli",
      "packageTargetKind": "package"
    }
  },
  "self": {}
}
"#,
    );
    let mut state_contents = fs::read(workspace.state_file()).expect("state file should exist");
    state_contents.splice(0..0, [0xEF, 0xBB, 0xBF]);
    workspace.write_state_bytes(&state_contents);

    let output = run_agx(&workspace, &["--json", "--dry-run", "update", "--all"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["scope"], "all");
    assert_eq!(json["data"]["results"][0]["name"], "qoder");
    assert_eq!(json["data"]["results"][0]["status"], "planned");
}
