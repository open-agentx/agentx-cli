mod support;

use support::{TestWorkspace, run_agx, run_agx_with_env, stdout_json, stdout_json_lines};

#[test]
fn install_unknown_agent_returns_agent_not_found() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "install", "missing-agent"]);

    assert_eq!(output.status.code(), Some(3));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "AGENT_NOT_FOUND");
}

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
fn install_reports_already_installed_when_binary_exists() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");
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

    let output = run_agx(&workspace, &["--json", "install", "qoder"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["action"], "install");
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
fn install_explains_when_existing_binary_is_not_tracked() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = run_agx(&workspace, &["--json", "install", "qoder"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["installed"], true);
    assert_eq!(json["data"]["changed"], false);
    assert!(
        json["data"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("not tracked by AGX")
    );
}

#[test]
fn install_tracks_existing_bun_install_when_source_is_unambiguous() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_bun_agent_binary("qodercli");

    let output = run_agx(&workspace, &["--json", "install", "qoder"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["installed"], true);
    assert_eq!(json["data"]["changed"], true);
    assert_eq!(json["data"]["installState"]["installType"], "bun");
    assert_eq!(
        json["data"]["installState"]["packageName"],
        "@qoder-ai/qodercli"
    );
    assert!(
        json["data"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("now tracking the existing install")
    );
}

#[test]
fn install_tracks_existing_script_install_when_agent_supports_self_update() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("agent");

    let output = run_agx(&workspace, &["--json", "install", "cursor"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["installed"], true);
    assert_eq!(json["data"]["changed"], true);
    assert_eq!(json["data"]["installState"]["installType"], "script");
    assert_eq!(json["data"]["installState"]["command"], "agent update");
    assert!(
        json["data"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("now tracking the existing install")
    );
}

#[test]
fn install_ndjson_emits_single_result_event() {
    let workspace = TestWorkspace::new();
    let output = run_agx(
        &workspace,
        &["--output", "ndjson", "--dry-run", "install", "qoder"],
    );

    assert!(output.status.success());
    let lines = stdout_json_lines(&output);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["type"], "result");
    assert_eq!(lines[0]["action"], "install");
    assert_eq!(lines[0]["meta"]["mode"], "ndjson");
    assert_eq!(lines[0]["data"]["data"]["agent"]["name"], "qoder");
}

#[test]
fn install_manual_only_agent_requires_manual_action() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "install", "jcode"]);

    assert_eq!(output.status.code(), Some(8));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "MANUAL_ACTION_REQUIRED");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("does not have a managed npm or Bun package yet")
    );
}

#[test]
fn ensure_reports_already_installed_when_binary_exists() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");
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
fn ensure_explains_when_existing_binary_is_not_tracked() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = run_agx(&workspace, &["--json", "ensure", "qoder"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["installed"], true);
    assert_eq!(json["data"]["changed"], false);
    assert!(
        json["data"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("not tracked by AGX")
    );
}

#[test]
fn ensure_tracks_existing_npm_install_when_source_is_unambiguous() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_npm_agent_binary("qodercli");

    let output = run_agx(&workspace, &["--json", "ensure", "qoder"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["installed"], true);
    assert_eq!(json["data"]["changed"], true);
    assert_eq!(json["data"]["installState"]["installType"], "npm");
    assert_eq!(
        json["data"]["installState"]["packageName"],
        "@qoder-ai/qodercli"
    );
    assert!(
        json["data"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("now tracking the existing install")
    );
}

#[test]
fn ensure_tracks_existing_script_install_when_agent_supports_self_update() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("agent");

    let output = run_agx(&workspace, &["--json", "ensure", "cursor"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["installed"], true);
    assert_eq!(json["data"]["changed"], true);
    assert_eq!(json["data"]["installState"]["installType"], "script");
    assert_eq!(json["data"]["installState"]["command"], "agent update");
    assert!(
        json["data"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("now tracking the existing install")
    );
}

#[test]
fn ensure_unknown_agent_returns_agent_not_found() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "ensure", "missing-agent"]);

    assert_eq!(output.status.code(), Some(3));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "AGENT_NOT_FOUND");
}

#[test]
fn ensure_manual_only_agent_requires_manual_action() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "ensure", "jcode"]);

    assert_eq!(output.status.code(), Some(8));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "MANUAL_ACTION_REQUIRED");
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
fn uninstall_unknown_agent_returns_agent_not_found() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "uninstall", "missing-agent"]);

    assert_eq!(output.status.code(), Some(3));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "AGENT_NOT_FOUND");
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
fn uninstall_requires_manual_action_when_tracked_package_is_missing() {
    let workspace = TestWorkspace::new();
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {
    "cursor": {
      "agentName": "cursor",
      "installType": "script",
      "command": "manual-install"
    }
  },
  "self": {}
}
"#,
    );

    let output = run_agx(&workspace, &["--json", "uninstall", "cursor"]);

    assert_eq!(output.status.code(), Some(8));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "MANUAL_ACTION_REQUIRED");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("does not have a managed package recorded")
    );
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

#[test]
fn update_requires_agent_name_without_all() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "update"]);

    assert_eq!(output.status.code(), Some(2));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
}

#[test]
fn update_unknown_agent_returns_agent_not_found() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "update", "missing-agent"]);

    assert_eq!(output.status.code(), Some(3));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "AGENT_NOT_FOUND");
}

#[test]
fn update_returns_agent_not_installed_when_state_is_missing() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "update", "qoder"]);

    assert_eq!(output.status.code(), Some(4));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "AGENT_NOT_INSTALLED");
}

#[test]
fn update_all_reports_manual_required_for_unknown_tracked_agent() {
    let workspace = TestWorkspace::new();
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {
    "legacy-agent": {
      "agentName": "legacy-agent",
      "installType": "bun",
      "packageName": "@legacy/agent",
      "packageTargetKind": "package"
    }
  },
  "self": {}
}
"#,
    );

    let output = run_agx(&workspace, &["--json", "update", "--all"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["scope"], "all");
    assert_eq!(json["data"]["results"][0]["name"], "legacy-agent");
    assert_eq!(json["data"]["results"][0]["status"], "manual-required");
}

#[test]
fn update_ndjson_emits_single_result_event() {
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

    let output = run_agx(
        &workspace,
        &["--output", "ndjson", "--dry-run", "update", "qoder"],
    );

    assert!(output.status.success());
    let lines = stdout_json_lines(&output);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["type"], "result");
    assert_eq!(lines[0]["action"], "update");
    assert_eq!(lines[0]["meta"]["mode"], "ndjson");
    assert_eq!(lines[0]["data"]["data"]["results"][0]["status"], "planned");
}

#[test]
fn update_single_reports_up_to_date_when_versions_match() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");
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

    let output = run_agx_with_env(
        &workspace,
        &["--json", "update", "qoder"],
        &[("AGX_TEST_LATEST_PACKAGE__QODER_AI_QODERCLI", "0.1.0")],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["results"][0]["status"], "up-to-date");
    assert_eq!(json["data"]["results"][0]["installedVersion"], "0.1.0");
    assert_eq!(json["data"]["results"][0]["latestVersion"], "0.1.0");
}

#[test]
fn update_all_marks_untracked_path_installs_as_manual_required() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = run_agx(&workspace, &["--json", "update", "--all"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let result = json["data"]["results"]
        .as_array()
        .expect("results should be an array")
        .iter()
        .find(|entry| entry["name"] == "qoder")
        .expect("qoder result should exist");
    assert_eq!(result["status"], "manual-required");
    assert!(
        result["message"]
            .as_str()
            .expect("message should exist")
            .contains("detected in PATH but not tracked")
    );
    assert!(
        result["hint"]
            .as_str()
            .expect("hint should exist")
            .contains("agx inspect qoder --json")
    );
}

#[test]
fn update_all_includes_tracked_script_installs_via_self_update() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {
    "qoder": {
      "agentName": "qoder",
      "installType": "script",
      "command": "qodercli update"
    }
  },
  "self": {}
}
"#,
    );

    let output = run_agx_with_env(
        &workspace,
        &["--json", "update", "--all"],
        &[("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1")],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    let result = json["data"]["results"]
        .as_array()
        .expect("results should be an array")
        .iter()
        .find(|entry| entry["name"] == "qoder")
        .expect("qoder result should exist");
    assert_eq!(result["status"], "updated");
    assert_eq!(result["strategy"], "self-update");
}

#[test]
fn update_single_uses_managed_update_for_untracked_known_package_agents() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = run_agx_with_env(
        &workspace,
        &["--json", "update", "qoder"],
        &[
            ("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1"),
            ("AGX_TEST_LATEST_PACKAGE__QODER_AI_QODERCLI", "0.2.0"),
        ],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["results"][0]["status"], "updated");
    assert_eq!(json["data"]["results"][0]["strategy"], "managed/bun");
}

#[test]
fn update_single_self_update_failure_returns_hint() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {
    "qoder": {
      "agentName": "qoder",
      "installType": "script",
      "command": "qodercli update"
    }
  },
  "self": {}
}
"#,
    );

    let output = run_agx_with_env(
        &workspace,
        &["--json", "update", "qoder"],
        &[("AGX_TEST_LATEST_PACKAGE__QODER_AI_QODERCLI", "0.2.0")],
    );

    assert_eq!(output.status.code(), Some(1));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "UPDATE_FAILED");
    assert_eq!(json["data"]["results"][0]["status"], "failed");
    assert!(
        json["data"]["results"][0]["hint"]
            .as_str()
            .expect("hint should exist")
            .contains("Try running qodercli update directly")
    );
}
