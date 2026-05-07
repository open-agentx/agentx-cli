mod support;

use support::{TestWorkspace, run_agx, stdout_json};

#[test]
fn install_dry_run_returns_planned_managed_state() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "--dry-run", "install", "qoder"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["action"], "install");
    assert_eq!(json["data"]["installed"], false);
    assert_eq!(json["data"]["installState"]["installType"], "bun");
    assert_eq!(
        json["data"]["installState"]["packageName"],
        "@qoder-ai/qodercli"
    );
}

#[test]
fn ensure_reports_already_installed_when_binary_exists() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = run_agx(&workspace, &["--json", "ensure", "qoder"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["action"], "ensure");
    assert_eq!(json["data"]["installed"], true);
    assert_eq!(json["data"]["changed"], false);
    assert!(
        json["data"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("already installed")
    );
}

#[test]
fn uninstall_dry_run_uses_recorded_managed_package() {
    let workspace = TestWorkspace::new();
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {
    "qoder": {
      "agentName": "qoder",
      "installType": "npm",
      "packageName": "@qoder-ai/qodercli",
      "packageTargetKind": "package"
    }
  },
  "self": {}
}
"#,
    );

    let output = run_agx(&workspace, &["--json", "--dry-run", "uninstall", "qoder"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["action"], "uninstall");
    assert_eq!(json["data"]["installed"], true);
    assert!(
        json["data"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("npm uninstall -g @qoder-ai/qodercli")
    );
}

#[test]
fn uninstall_returns_agent_not_installed_when_state_is_missing() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "uninstall", "qoder"]);

    assert_eq!(output.status.code(), Some(4));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "AGENT_NOT_INSTALLED");
}

#[test]
fn update_single_dry_run_uses_recorded_install_source() {
    let workspace = TestWorkspace::new();
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {
    "qoder": {
      "agentName": "qoder",
      "installType": "npm",
      "packageName": "@qoder-ai/qodercli",
      "packageTargetKind": "package"
    }
  },
  "self": {}
}
"#,
    );

    let output = run_agx(&workspace, &["--json", "--dry-run", "update", "qoder"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["scope"], "single");
    assert_eq!(json["data"]["results"][0]["strategy"], "managed/npm");
    assert_eq!(json["data"]["results"][0]["status"], "planned");
}
