mod support;

use std::fs;

use support::{TestWorkspace, run_agx, stdout_json};

#[test]
fn capabilities_json_reports_controlled_installer_availability() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("bun");

    let output = run_agx(&workspace, &["--json", "capabilities"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["action"], "capabilities");
    assert_eq!(json["data"]["installers"]["bun"]["available"], true);
    assert_eq!(json["data"]["installers"]["npm"]["available"], false);
    assert_eq!(
        json["data"]["features"]["execInstallPolicies"][1],
        "if-missing"
    );
    assert_eq!(json["data"]["outputModes"][2], "ndjson");
}

#[test]
fn doctor_json_reports_recorded_install_source_and_missing_installers() {
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

    let output = run_agx(&workspace, &["--json", "doctor"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["action"], "doctor");
    assert_eq!(json["data"]["installSource"]["kind"], "bun");
    assert_eq!(json["data"]["installSource"]["confidence"], "recorded");
    assert_eq!(json["data"]["ok"], false);
    assert_eq!(json["data"]["checks"][1]["name"], "bun");
    assert_eq!(json["data"]["checks"][1]["status"], "warn");
    assert_eq!(json["data"]["checks"][2]["name"], "npm");
    assert_eq!(json["data"]["checks"][2]["status"], "warn");
}

#[test]
fn doctor_json_warns_for_invalid_config_and_stale_lock() {
    let workspace = TestWorkspace::new();
    workspace.write_config_bytes(b"{not-valid-json}\n");
    fs::write(workspace.config_dir().join("state-lock.lock"), b"locked")
        .expect("lock file should be writable");

    let output = run_agx(&workspace, &["--json", "doctor"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let checks = json["data"]["checks"]
        .as_array()
        .expect("checks should be an array");

    assert!(
        checks
            .iter()
            .any(|check| { check["name"] == "config" && check["status"] == "warn" })
    );
    assert!(
        checks
            .iter()
            .any(|check| { check["name"] == "state-lock" && check["status"] == "warn" })
    );
}
