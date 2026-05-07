mod support;

use std::fs;

use support::{TestWorkspace, run_agx, stdout_json};

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
