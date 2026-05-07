mod support;

use std::fs;

use support::{TestWorkspace, run_agx, stdout_json};

#[test]
fn config_reset_restores_defaults() {
    let workspace = TestWorkspace::new();
    workspace.write_config_bytes(b"{\"defaultPackageManager\":\"npm\"}\n");

    let output = run_agx(&workspace, &["--json", "config", "reset"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["action"], "reset");
    assert_eq!(json["data"]["config"]["defaultPackageManager"], "bun");

    let stored = fs::read_to_string(workspace.config_file()).expect("config should exist");
    assert!(stored.contains("\"defaultPackageManager\": \"bun\""));
}

#[test]
fn config_get_requires_a_key() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "config", "get"]);

    assert_eq!(output.status.code(), Some(2));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("Please specify a key")
    );
}

#[test]
fn config_rejects_invalid_values_with_invalid_argument() {
    let workspace = TestWorkspace::new();
    let output = run_agx(
        &workspace,
        &["--json", "config", "set", "selfUpdateChannel", "nightly"],
    );

    assert_eq!(output.status.code(), Some(2));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
}

#[test]
fn config_rejects_unknown_actions() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "config", "delete"]);

    assert_eq!(output.status.code(), Some(2));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("Unknown action")
    );
}

#[test]
fn upgrade_dry_run_uses_recorded_bun_install_source() {
    let workspace = TestWorkspace::new();
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {},
  "self": {
    "installSource": "bun"
  }
}
"#,
    );

    let output = run_agx(&workspace, &["--json", "--dry-run", "upgrade"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["installSource"], "bun");
    assert_eq!(json["data"]["status"], "planned");
    assert_eq!(json["data"]["command"][0], "bun");
}

#[test]
fn upgrade_dry_run_uses_recorded_npm_install_source() {
    let workspace = TestWorkspace::new();
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {},
  "self": {
    "installSource": "npm"
  }
}
"#,
    );

    let output = run_agx(&workspace, &["--json", "--dry-run", "upgrade"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["installSource"], "npm");
    assert_eq!(json["data"]["status"], "planned");
    assert_eq!(json["data"]["command"][0], "npm");
}

#[test]
fn upgrade_rejects_source_build_without_recorded_install_source() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "upgrade"]);

    assert_eq!(output.status.code(), Some(8));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "MANUAL_ACTION_REQUIRED");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("source build")
    );
}

#[test]
fn upgrade_rejects_unknown_recorded_install_source() {
    let workspace = TestWorkspace::new();
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {},
  "self": {
    "installSource": "mystery"
  }
}
"#,
    );

    let output = run_agx(&workspace, &["--json", "upgrade"]);

    assert_eq!(output.status.code(), Some(8));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "MANUAL_ACTION_REQUIRED");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("could not determine its install source")
    );
}
