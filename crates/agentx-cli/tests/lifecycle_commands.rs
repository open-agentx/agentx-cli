mod support;

use std::fs;

use support::{
    TestWorkspace, run_agx, run_agx_with_env, stdout_json, stdout_json_lines, stdout_text,
};

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
    assert_eq!(json["warnings"][0]["code"], "DRY_RUN");
    assert_eq!(
        json["data"]["installState"]["packageName"],
        "@qoder-ai/qodercli"
    );
}

#[test]
fn install_dry_run_supports_cargo_managed_agents() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "--dry-run", "install", "vtcode"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["installState"]["installType"], "cargo");
    assert_eq!(json["data"]["installState"]["packageName"], "vtcode");
    assert!(
        json["data"]["message"]
            .as_str()
            .expect("dry-run message should exist")
            .contains("cargo install vtcode")
    );
}

#[test]
fn install_cargo_agent_records_managed_state() {
    let workspace = TestWorkspace::new();

    let output = run_agx_with_env(
        &workspace,
        &["--json", "install", "vtcode"],
        &[("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1")],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["installed"], true);
    assert_eq!(json["data"]["installState"]["installType"], "cargo");
    assert_eq!(json["data"]["installState"]["packageName"], "vtcode");
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
    assert_eq!(json["warnings"][0]["code"], "ALREADY_INSTALLED");
    assert!(
        json["warnings"][0]["message"]
            .as_str()
            .expect("warning should exist")
            .contains("already installed")
    );
}

#[test]
fn install_successfully_records_managed_install_state() {
    let workspace = TestWorkspace::new();

    let output = run_agx_with_env(
        &workspace,
        &["--json", "install", "qoder"],
        &[("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1")],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["installed"], true);
    assert_eq!(json["data"]["changed"], true);
    assert_eq!(json["data"]["installState"]["installType"], "bun");

    let state = fs::read_to_string(workspace.state_file()).expect("state file should exist");
    assert!(state.contains("\"qoder\""));
    assert!(state.contains("\"installType\": \"bun\""));
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
    assert_eq!(json["warnings"][0]["code"], "UNTRACKED_EXISTING_INSTALL");
    assert!(
        json["warnings"][0]["message"]
            .as_str()
            .expect("warning should exist")
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
    assert_eq!(json["warnings"][0]["code"], "TRACKED_EXISTING_INSTALL");
    assert!(
        json["warnings"][0]["message"]
            .as_str()
            .expect("warning should exist")
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
    assert_eq!(json["warnings"][0]["code"], "TRACKED_EXISTING_INSTALL");
    assert!(
        json["warnings"][0]["message"]
            .as_str()
            .expect("warning should exist")
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
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["type"], "started");
    assert_eq!(lines[0]["action"], "install");
    assert_eq!(lines[0]["data"]["agent"], "qoder");
    assert_eq!(lines[1]["type"], "result");
    assert_eq!(lines[1]["action"], "install");
    assert_eq!(lines[1]["meta"]["mode"], "ndjson");
    assert_eq!(lines[1]["data"]["data"]["agent"]["name"], "qoder");
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
            .contains("does not have a managed npm, Bun, or Cargo package yet")
    );
}

#[test]
fn install_multiple_agents_returns_batch_json_summary() {
    let workspace = TestWorkspace::new();

    let output = run_agx_with_env(
        &workspace,
        &["--json", "install", "qoder", "reasonix"],
        &[("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1")],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["action"], "install");
    assert_eq!(json["data"]["scope"], "batch");
    assert_eq!(json["data"]["summary"]["installed"], 2);
    let results = json["data"]["results"]
        .as_array()
        .expect("results should be an array");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["status"], "installed");
    assert_eq!(results[1]["status"], "installed");
    assert_eq!(results[1]["agent"]["name"], "reasonix");
}

#[test]
fn install_multiple_agents_continues_after_failure_and_reports_aggregate_error() {
    let workspace = TestWorkspace::new();

    let output = run_agx_with_env(
        &workspace,
        &["--json", "install", "qoder", "missing-agent"],
        &[("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1")],
    );

    assert_eq!(output.status.code(), Some(1));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "INSTALL_FAILED");
    assert_eq!(json["data"]["scope"], "batch");
    let results = json["data"]["results"]
        .as_array()
        .expect("results should be an array");
    assert_eq!(results[0]["status"], "installed");
    assert_eq!(results[1]["status"], "failed");
    assert_eq!(results[1]["input"], "missing-agent");
}

#[test]
fn install_multiple_agents_ndjson_emits_started_progress_and_result_events() {
    let workspace = TestWorkspace::new();

    let output = run_agx_with_env(
        &workspace,
        &["--output", "ndjson", "install", "qoder", "reasonix"],
        &[("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1")],
    );

    assert!(output.status.success());
    let lines = stdout_json_lines(&output);
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0]["type"], "started");
    assert_eq!(lines[0]["data"]["scope"], "batch");
    assert_eq!(lines[1]["type"], "progress");
    assert_eq!(lines[1]["data"]["agent"]["name"], "qoder");
    assert_eq!(lines[2]["type"], "progress");
    assert_eq!(lines[2]["data"]["agent"]["name"], "reasonix");
    assert_eq!(lines[3]["type"], "result");
    assert_eq!(lines[3]["data"]["data"]["scope"], "batch");
}

#[test]
fn install_multiple_agents_human_output_includes_summary() {
    let workspace = TestWorkspace::new();

    let output = run_agx_with_env(
        &workspace,
        &["install", "qoder", "reasonix"],
        &[("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1")],
    );

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Qoder CLI installed successfully"));
    assert!(stdout.contains("Reasonix installed successfully"));
    assert!(stdout.contains("Summary: installed 2"));
}

#[test]
fn install_returns_install_failed_when_external_install_command_fails() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "install", "qoder"]);

    assert_eq!(output.status.code(), Some(1));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "INSTALL_FAILED");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("Failed to run `bun add -g @qoder-ai/qodercli`")
    );
}

#[test]
fn install_returns_resource_locked_when_lifecycle_lock_is_held() {
    let workspace = TestWorkspace::new();
    fs::create_dir_all(workspace.config_dir()).expect("config dir should exist");
    fs::write(
        workspace.config_dir().join("agent-lifecycle.lock"),
        b"locked\n",
    )
    .expect("lock file should be written");

    let output = run_agx(&workspace, &["--json", "install", "qoder"]);

    assert_eq!(output.status.code(), Some(9));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "RESOURCE_LOCKED");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("agent lifecycle")
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
    assert_eq!(json["warnings"][0]["code"], "ALREADY_INSTALLED");
    assert!(
        json["warnings"][0]["message"]
            .as_str()
            .expect("warning should exist")
            .contains("already installed")
    );
}

#[test]
fn ensure_successfully_records_managed_install_state() {
    let workspace = TestWorkspace::new();

    let output = run_agx_with_env(
        &workspace,
        &["--json", "ensure", "qoder"],
        &[("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1")],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["installed"], true);
    assert_eq!(json["data"]["changed"], true);
    assert_eq!(json["data"]["installState"]["installType"], "bun");

    let state = fs::read_to_string(workspace.state_file()).expect("state file should exist");
    assert!(state.contains("\"qoder\""));
    assert!(state.contains("\"installType\": \"bun\""));
}

#[test]
fn ensure_human_output_shows_install_progress_and_success() {
    let workspace = TestWorkspace::new();

    let output = run_agx_with_env(
        &workspace,
        &["ensure", "qoder"],
        &[("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1")],
    );

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Installing Qoder CLI..."));
    assert!(stdout.contains("Qoder CLI is now installed."));
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
    assert_eq!(json["warnings"][0]["code"], "UNTRACKED_EXISTING_INSTALL");
    assert!(
        json["warnings"][0]["message"]
            .as_str()
            .expect("warning should exist")
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
    assert_eq!(json["warnings"][0]["code"], "TRACKED_EXISTING_INSTALL");
    assert!(
        json["warnings"][0]["message"]
            .as_str()
            .expect("warning should exist")
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
    assert_eq!(json["warnings"][0]["code"], "TRACKED_EXISTING_INSTALL");
    assert!(
        json["warnings"][0]["message"]
            .as_str()
            .expect("warning should exist")
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
    assert_eq!(json["warnings"][0]["code"], "DRY_RUN");
    assert!(
        json["data"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("npm uninstall -g @qoder-ai/qodercli")
    );
}

#[test]
fn uninstall_dry_run_uses_recorded_cargo_package() {
    let workspace = TestWorkspace::new();
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {
    "vtcode": {
      "agentName": "vtcode",
      "installType": "cargo",
      "packageName": "vtcode",
      "packageTargetKind": "package"
    }
  },
  "self": {}
}
"#,
    );

    let output = run_agx(&workspace, &["--json", "--dry-run", "uninstall", "vtcode"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert!(
        json["data"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("cargo uninstall vtcode")
    );
}

#[test]
fn uninstall_ndjson_emits_started_and_result_events() {
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
        &["--output", "ndjson", "--dry-run", "uninstall", "qoder"],
    );

    assert!(output.status.success());
    let lines = stdout_json_lines(&output);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["type"], "started");
    assert_eq!(lines[0]["action"], "uninstall");
    assert_eq!(lines[0]["data"]["agent"], "qoder");
    assert_eq!(lines[1]["type"], "result");
    assert_eq!(lines[1]["action"], "uninstall");
    assert_eq!(lines[1]["data"]["data"]["agent"]["name"], "qoder");
}

#[test]
fn uninstall_successfully_removes_recorded_install_state() {
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

    let output = run_agx_with_env(
        &workspace,
        &["--json", "uninstall", "qoder"],
        &[("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1")],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["installed"], false);
    assert_eq!(json["data"]["changed"], true);

    let state = fs::read_to_string(workspace.state_file()).expect("state file should exist");
    assert!(!state.contains("\"qoder\""));
}

#[test]
fn uninstall_human_output_shows_progress_and_success() {
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

    let output = run_agx_with_env(
        &workspace,
        &["uninstall", "qoder"],
        &[("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1")],
    );

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Uninstalling Qoder CLI..."));
    assert!(stdout.contains("Qoder CLI uninstalled successfully!"));
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
fn uninstall_returns_uninstall_failed_when_external_command_fails() {
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

    let output = run_agx(&workspace, &["--json", "uninstall", "qoder"]);

    assert_eq!(output.status.code(), Some(1));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "UNINSTALL_FAILED");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("Failed to run `npm uninstall -g @qoder-ai/qodercli`")
    );
}

#[test]
fn uninstall_returns_resource_locked_when_lifecycle_lock_is_held() {
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
    fs::create_dir_all(workspace.config_dir()).expect("config dir should exist");
    fs::write(
        workspace.config_dir().join("agent-lifecycle.lock"),
        b"locked\n",
    )
    .expect("lock file should be written");

    let output = run_agx(&workspace, &["--json", "uninstall", "qoder"]);

    assert_eq!(output.status.code(), Some(9));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "RESOURCE_LOCKED");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("agent lifecycle")
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
fn update_single_npm_latest_major_uses_npm_install_latest() {
    let workspace = TestWorkspace::new();
    let capture_path = workspace.root().join("commands.log");
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

    let capture = capture_path.to_string_lossy().into_owned();
    let output = run_agx_with_env(
        &workspace,
        &["--json", "update", "qoder"],
        &[
            ("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1"),
            ("AGX_TEST_CAPTURE_COMMAND_PATH", &capture),
            ("AGX_TEST_LATEST_PACKAGE__QODER_AI_QODERCLI", "0.2.0"),
        ],
    );

    assert!(output.status.success());
    let captured = fs::read_to_string(capture_path).expect("capture file should exist");
    assert!(captured.contains("npm install -g @qoder-ai/qodercli@latest"));
}

#[test]
fn update_single_npm_respect_semver_uses_npm_update() {
    let workspace = TestWorkspace::new();
    let capture_path = workspace.root().join("commands.log");
    workspace.write_config_bytes(br#"{"npmBunUpdateStrategy":"respect-semver"}"#);
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

    let capture = capture_path.to_string_lossy().into_owned();
    let output = run_agx_with_env(
        &workspace,
        &["--json", "update", "qoder"],
        &[
            ("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1"),
            ("AGX_TEST_CAPTURE_COMMAND_PATH", &capture),
            ("AGX_TEST_LATEST_PACKAGE__QODER_AI_QODERCLI", "0.2.0"),
        ],
    );

    assert!(output.status.success());
    let captured = fs::read_to_string(capture_path).expect("capture file should exist");
    assert!(captured.contains("npm update -g @qoder-ai/qodercli"));
    assert!(!captured.contains("@latest"));
}

#[test]
fn update_single_bun_latest_major_uses_bun_update_latest() {
    let workspace = TestWorkspace::new();
    let capture_path = workspace.root().join("commands.log");
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

    let capture = capture_path.to_string_lossy().into_owned();
    let output = run_agx_with_env(
        &workspace,
        &["--json", "update", "qoder"],
        &[
            ("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1"),
            ("AGX_TEST_CAPTURE_COMMAND_PATH", &capture),
            ("AGX_TEST_LATEST_PACKAGE__QODER_AI_QODERCLI", "0.2.0"),
        ],
    );

    assert!(output.status.success());
    let captured = fs::read_to_string(capture_path).expect("capture file should exist");
    assert!(captured.contains("bun update -g --latest @qoder-ai/qodercli"));
}

#[test]
fn update_single_cargo_uses_force_install_with_recorded_args() {
    let workspace = TestWorkspace::new();
    let capture_path = workspace.root().join("commands.log");
    workspace.install_fake_agent_binary("deepseek");
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {
    "deepseek": {
      "agentName": "deepseek",
      "installType": "cargo",
      "packageName": "deepseek-tui-cli",
      "packageTargetKind": "package",
      "packageInstallArgs": ["--locked"]
    }
  },
  "self": {}
}
"#,
    );

    let capture = capture_path.to_string_lossy().into_owned();
    let output = run_agx_with_env(
        &workspace,
        &["--json", "update", "deepseek"],
        &[
            ("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1"),
            ("AGX_TEST_CAPTURE_COMMAND_PATH", &capture),
            ("AGX_TEST_LATEST_PACKAGE_DEEPSEEK_TUI_CLI", "0.2.0"),
        ],
    );

    assert!(output.status.success());
    let captured = fs::read_to_string(capture_path).expect("capture file should exist");
    assert!(captured.contains("cargo install deepseek-tui-cli --force --locked"));
}

#[test]
fn update_single_bun_respect_semver_uses_bun_update_without_latest() {
    let workspace = TestWorkspace::new();
    let capture_path = workspace.root().join("commands.log");
    workspace.write_config_bytes(br#"{"npmBunUpdateStrategy":"respect-semver"}"#);
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

    let capture = capture_path.to_string_lossy().into_owned();
    let output = run_agx_with_env(
        &workspace,
        &["--json", "update", "qoder"],
        &[
            ("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1"),
            ("AGX_TEST_CAPTURE_COMMAND_PATH", &capture),
            ("AGX_TEST_LATEST_PACKAGE__QODER_AI_QODERCLI", "0.2.0"),
        ],
    );

    assert!(output.status.success());
    let captured = fs::read_to_string(capture_path).expect("capture file should exist");
    assert!(captured.contains("bun update -g @qoder-ai/qodercli"));
    assert!(!captured.contains("--latest"));
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
fn update_all_human_output_includes_summary_counts() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");
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

    let output = run_agx(&workspace, &["update", "--all"]);

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Qoder CLI: manual action required."));
    assert!(stdout.contains("legacy-agent: manual action required."));
    assert!(stdout.contains("Summary: manual 2"));
}

#[test]
fn update_single_human_output_reports_successful_managed_update_with_version_transition() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = run_agx_with_env(
        &workspace,
        &["update", "qoder"],
        &[
            ("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1"),
            ("AGX_TEST_LATEST_PACKAGE__QODER_AI_QODERCLI", "0.2.0"),
        ],
    );

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Updating Qoder CLI via managed/bun... (0.1.0 -> 0.2.0)"));
    assert!(stdout.contains("Qoder CLI updated successfully!"));
}

#[test]
fn update_all_batches_bun_managed_updates_into_one_command() {
    let workspace = TestWorkspace::new();
    let capture_path = workspace.root().join("commands.log");
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {
    "codex": {
      "agentName": "codex",
      "installType": "bun",
      "packageName": "@openai/codex",
      "packageTargetKind": "package"
    },
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

    let capture = capture_path.to_string_lossy().into_owned();
    let output = run_agx_with_env(
        &workspace,
        &["--json", "update", "--all"],
        &[
            ("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1"),
            ("AGX_TEST_CAPTURE_COMMAND_PATH", &capture),
            ("AGX_TEST_LATEST_PACKAGE__OPENAI_CODEX", "0.2.0"),
            ("AGX_TEST_LATEST_PACKAGE__QODER_AI_QODERCLI", "0.2.0"),
        ],
    );

    assert!(output.status.success());
    let captured = fs::read_to_string(capture_path).expect("capture file should exist");
    let lines = captured
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("bun update -g --latest"));
    assert!(lines[0].contains("@openai/codex"));
    assert!(lines[0].contains("@qoder-ai/qodercli"));
}

#[test]
fn update_all_batches_npm_respect_semver_updates_into_one_command() {
    let workspace = TestWorkspace::new();
    let capture_path = workspace.root().join("commands.log");
    workspace.write_config_bytes(br#"{"npmBunUpdateStrategy":"respect-semver"}"#);
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {
    "codex": {
      "agentName": "codex",
      "installType": "npm",
      "packageName": "@openai/codex",
      "packageTargetKind": "package"
    },
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

    let capture = capture_path.to_string_lossy().into_owned();
    let output = run_agx_with_env(
        &workspace,
        &["--json", "update", "--all"],
        &[
            ("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1"),
            ("AGX_TEST_CAPTURE_COMMAND_PATH", &capture),
            ("AGX_TEST_LATEST_PACKAGE__OPENAI_CODEX", "0.2.0"),
            ("AGX_TEST_LATEST_PACKAGE__QODER_AI_QODERCLI", "0.2.0"),
        ],
    );

    assert!(output.status.success());
    let captured = fs::read_to_string(capture_path).expect("capture file should exist");
    let lines = captured
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("npm update -g"));
    assert!(lines[0].contains("@openai/codex"));
    assert!(lines[0].contains("@qoder-ai/qodercli"));
    assert!(!lines[0].contains("@latest"));
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
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0]["type"], "started");
    assert_eq!(lines[0]["action"], "update");
    assert_eq!(lines[0]["data"]["scope"], "single");
    assert_eq!(lines[1]["type"], "progress");
    assert_eq!(lines[1]["data"]["status"], "planned");
    assert_eq!(lines[2]["type"], "result");
    assert_eq!(lines[2]["action"], "update");
    assert_eq!(lines[2]["meta"]["mode"], "ndjson");
    assert_eq!(lines[2]["data"]["data"]["results"][0]["status"], "planned");
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
    assert_eq!(result["status"], "up-to-date");
    assert_eq!(result["strategy"], "self-update");
}

#[test]
fn update_all_reports_tracked_script_installs_as_up_to_date_when_version_does_not_change() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("agent");
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {
    "cursor": {
      "agentName": "cursor",
      "installType": "script",
      "command": "agent update"
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
        .find(|entry| entry["name"] == "cursor")
        .expect("cursor result should exist");
    assert_eq!(result["status"], "up-to-date");
    assert_eq!(result["installedVersion"], "0.1.0");
    assert_eq!(result["latestVersion"], "0.1.0");
}

#[test]
fn update_all_uses_managed_package_versions_for_up_to_date_detection() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");
    workspace.install_fake_agent_binary("pi");
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {
    "qoder": {
      "agentName": "qoder",
      "installType": "bun",
      "packageName": "@qoder-ai/qodercli",
      "packageTargetKind": "package"
    },
    "pi": {
      "agentName": "pi",
      "installType": "bun",
      "packageName": "@mariozechner/pi-coding-agent",
      "packageTargetKind": "package"
    }
  },
  "self": {}
}
"#,
    );

    let output = run_agx_with_env(
        &workspace,
        &["--json", "update", "--all"],
        &[
            ("AGX_TEST_MANAGED_VERSION__QODER_AI_QODERCLI", "1.0.43"),
            (
                "AGX_TEST_MANAGED_VERSION__MARIOZECHNER_PI_CODING_AGENT",
                "0.73.1",
            ),
            ("AGX_TEST_LATEST_PACKAGE__QODER_AI_QODERCLI", "1.0.43"),
            (
                "AGX_TEST_LATEST_PACKAGE__MARIOZECHNER_PI_CODING_AGENT",
                "0.73.1",
            ),
        ],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    let results = json["data"]["results"]
        .as_array()
        .expect("results should be an array");

    let qoder = results
        .iter()
        .find(|entry| entry["name"] == "qoder")
        .expect("qoder result should exist");
    let pi = results
        .iter()
        .find(|entry| entry["name"] == "pi")
        .expect("pi result should exist");

    assert_eq!(qoder["status"], "up-to-date");
    assert_eq!(qoder["installedVersion"], "1.0.43");
    assert_eq!(pi["status"], "up-to-date");
    assert_eq!(pi["installedVersion"], "0.73.1");
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
fn update_single_self_update_reports_up_to_date_when_version_does_not_change() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("agent");
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {
    "cursor": {
      "agentName": "cursor",
      "installType": "script",
      "command": "agent update"
    }
  },
  "self": {}
}
"#,
    );

    let output = run_agx_with_env(
        &workspace,
        &["--json", "update", "cursor"],
        &[("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1")],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["results"][0]["status"], "up-to-date");
    assert_eq!(json["data"]["results"][0]["installedVersion"], "0.1.0");
    assert_eq!(json["data"]["results"][0]["latestVersion"], "0.1.0");
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

#[test]
fn update_single_human_output_reports_failure_with_next_step() {
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
        &["update", "qoder"],
        &[("AGX_TEST_LATEST_PACKAGE__QODER_AI_QODERCLI", "0.2.0")],
    );

    assert_eq!(output.status.code(), Some(1));
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Updating Qoder CLI via self-update... (0.1.0 -> 0.2.0)"));
    assert!(stdout.contains("Failed to update Qoder CLI."));
    assert!(stdout.contains("Next step: Try running qodercli update directly."));
}

#[test]
fn update_single_returns_resource_locked_when_lifecycle_lock_is_held() {
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
    fs::create_dir_all(workspace.config_dir()).expect("config dir should exist");
    fs::write(
        workspace.config_dir().join("agent-lifecycle.lock"),
        b"locked\n",
    )
    .expect("lock file should be written");

    let output = run_agx(&workspace, &["--json", "update", "qoder"]);

    assert_eq!(output.status.code(), Some(9));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "RESOURCE_LOCKED");
    assert!(
        json["error"]["details"]["resource"]
            .as_str()
            .expect("resource should exist")
            .contains("agent-lifecycle.lock")
    );
    assert_eq!(json["data"]["results"][0]["status"], "locked");
    assert!(
        json["data"]["results"][0]["resource"]
            .as_str()
            .expect("resource should exist")
            .contains("agent-lifecycle.lock")
    );
}

#[test]
fn update_all_returns_resource_locked_when_lifecycle_lock_is_held() {
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
    fs::create_dir_all(workspace.config_dir()).expect("config dir should exist");
    fs::write(
        workspace.config_dir().join("agent-lifecycle.lock"),
        b"locked\n",
    )
    .expect("lock file should be written");

    let output = run_agx(&workspace, &["--json", "update", "--all"]);

    assert_eq!(output.status.code(), Some(9));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "RESOURCE_LOCKED");
    assert_eq!(json["data"]["results"][0]["status"], "locked");
    assert!(
        json["data"]["results"][0]["resource"]
            .as_str()
            .expect("resource should exist")
            .contains("agent-lifecycle.lock")
    );
    assert!(
        json["data"]["results"][0]["message"]
            .as_str()
            .expect("message should exist")
            .contains("agent lifecycle")
    );
}
