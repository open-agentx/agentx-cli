mod support;

use std::fs;

use support::{
    TestWorkspace, run_agx, run_agx_with_env, stdout_json, stdout_json_lines, stdout_text,
};

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
    assert_eq!(json["data"]["agents"][0], "auggie");
}

#[test]
fn capabilities_human_output_stays_readable() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["capabilities"]);

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("AGX Capabilities"));
    assert!(stdout.contains("agx capabilities --json"));
}

#[test]
fn capabilities_ndjson_emits_single_result_event() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--output", "ndjson", "capabilities"]);

    assert!(output.status.success());
    let lines = stdout_json_lines(&output);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["type"], "result");
    assert_eq!(lines[0]["action"], "capabilities");
    assert_eq!(lines[0]["meta"]["mode"], "ndjson");
    assert_eq!(lines[0]["data"]["data"]["outputModes"][1], "json");
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
fn doctor_json_exposes_paths() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "doctor"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert!(
        json["data"]["paths"]["stateFile"]
            .as_str()
            .expect("state file should exist")
            .contains(".quantex")
    );
    assert!(
        json["data"]["paths"]["configFile"]
            .as_str()
            .expect("config file should exist")
            .contains(".quantex")
    );
}

#[test]
fn doctor_json_reports_machine_actionable_self_issues() {
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

    let output = run_agx_with_env(
        &workspace,
        &["--json", "doctor"],
        &[("AGX_TEST_LATEST_VERSION", "0.2.0")],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    let issues = json["data"]["issues"]
        .as_array()
        .expect("issues should be an array");

    assert!(issues.iter().any(|issue| {
        issue["code"] == "SELF_INSTALLER_MISSING"
            && issue["suggestedAction"] == "restore-self-installer"
    }));
    assert!(issues.iter().any(|issue| {
        issue["code"] == "SELF_UPDATE_AVAILABLE" && issue["suggestedCommands"][0] == "agx upgrade"
    }));
}

#[test]
fn doctor_json_reports_untracked_agent_issue() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = run_agx(&workspace, &["--json", "doctor"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let issues = json["data"]["issues"]
        .as_array()
        .expect("issues should be an array");
    assert!(issues.iter().any(|issue| {
        issue["code"] == "AGENT_UNTRACKED_IN_PATH"
            && issue["suggestedCommands"][0] == "agx inspect qoder --json"
    }));
}

#[test]
fn doctor_json_reports_self_update_guidance_for_untracked_self_updating_agent() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("claude");

    let output = run_agx(&workspace, &["--json", "doctor"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let issues = json["data"]["issues"]
        .as_array()
        .expect("issues should be an array");
    assert!(issues.iter().any(|issue| {
        issue["code"] == "AGENT_MANUAL_UPDATE_REQUIRED"
            && issue["subject"]["name"] == "claude"
            && issue["suggestedAction"] == "run-agent-self-update"
            && issue["suggestedCommands"][0] == "claude update"
    }));
}

#[test]
fn doctor_json_reports_manual_update_guidance_for_untracked_agent_without_self_update() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("jcode");

    let output = run_agx(&workspace, &["--json", "doctor"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let issues = json["data"]["issues"]
        .as_array()
        .expect("issues should be an array");
    assert!(issues.iter().any(|issue| {
        issue["code"] == "AGENT_MANUAL_UPDATE_REQUIRED"
            && issue["subject"]["name"] == "jcode"
            && issue["suggestedAction"] == "follow-manual-agent-update"
            && issue["suggestedCommands"]
                .as_array()
                .expect("suggested commands should be an array")
                .is_empty()
    }));
}

#[test]
fn doctor_json_uses_source_build_heuristic_without_recorded_state() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "doctor"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["installSource"]["kind"], "source-build");
    assert_eq!(json["data"]["installSource"]["confidence"], "heuristic");
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

#[test]
fn doctor_human_output_includes_summary_and_check_names() {
    let workspace = TestWorkspace::new();
    workspace.write_config_bytes(b"{not-valid-json}\n");

    let output = run_agx(&workspace, &["doctor"]);

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("AGX runtime checks completed with warnings."));
    assert!(stdout.contains("Managed Installers:"));
    assert!(stdout.contains("AGX CLI:"));
    assert!(stdout.contains("Issues:"));
}

#[test]
fn doctor_human_output_shows_bun_recovery_for_outdated_self_install() {
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

    let output = run_agx_with_env(
        &workspace,
        &["doctor"],
        &[("AGX_TEST_LATEST_VERSION", "0.2.0")],
    );

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Recovery:"));
    assert!(stdout.contains("bun add -g agxctl@latest"));
}

#[test]
fn doctor_human_output_does_not_flag_self_as_outdated_when_latest_is_lower() {
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

    let output = run_agx_with_env(
        &workspace,
        &["doctor"],
        &[("AGX_TEST_LATEST_VERSION", "0.0.9")],
    );

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Latest:       0.0.9"));
    assert!(!stdout.contains("update available"));
}

#[test]
fn doctor_human_output_shows_binary_recovery_for_outdated_standalone_install() {
    let workspace = TestWorkspace::new();
    let executable = workspace.install_fake_self_binary();
    let manifest_path = workspace.root().join("manifest.json");
    fs::write(
        &manifest_path,
        standalone_manifest_json("0.2.0", &standalone_asset_name(), "placeholder"),
    )
    .expect("manifest should be written");
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {},
  "self": {
    "installSource": "standalone"
  }
}
"#,
    );

    let output = run_agx_with_env(
        &workspace,
        &["doctor"],
        &[
            (
                "AGX_TEST_SELF_EXECUTABLE_PATH",
                executable.to_string_lossy().as_ref(),
            ),
            (
                "AGX_TEST_STANDALONE_MANIFEST_PATH",
                manifest_path.to_string_lossy().as_ref(),
            ),
        ],
    );

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Recovery:"));
    assert!(stdout.contains("/releases/latest/download/agx-"));
}

#[test]
fn doctor_human_output_lists_installed_agents_with_versions() {
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

    let output = run_agx(&workspace, &["doctor"]);

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Qoder CLI"));
    assert!(stdout.contains("0.1.0"));
    assert!(stdout.contains("managed via bun (@qoder-ai/qodercli)"));
}

#[test]
fn doctor_human_output_marks_outdated_installed_agents() {
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

    let output = run_agx_with_env(
        &workspace,
        &["doctor"],
        &[("AGX_TEST_LATEST_PACKAGE__QODER_AI_QODERCLI", "1.0.0")],
    );

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("update available: 1.0.0"));
}

#[test]
fn doctor_human_output_reports_missing_managed_installers() {
    let workspace = TestWorkspace::new();

    let output = run_agx(&workspace, &["doctor"]);

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("No managed installer found"));
}

#[test]
fn doctor_human_output_shows_no_agents_installed_when_catalog_is_empty_on_path() {
    let workspace = TestWorkspace::new();

    let output = run_agx(&workspace, &["doctor"]);

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("No agents installed"));
}

#[test]
fn doctor_json_reports_self_auto_update_unavailable_for_source_builds() {
    let workspace = TestWorkspace::new();

    let output = run_agx_with_env(
        &workspace,
        &["--json", "doctor"],
        &[("AGX_TEST_LATEST_VERSION", "0.2.0")],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    let issues = json["data"]["issues"]
        .as_array()
        .expect("issues should be an array");
    assert!(issues.iter().any(|issue| {
        issue["code"] == "SELF_AUTO_UPDATE_UNAVAILABLE"
            && issue["suggestedAction"] == "reinstall-self-with-auto-update-source"
    }));
}

fn standalone_asset_name() -> String {
    match std::env::consts::OS {
        "windows" => match std::env::consts::ARCH {
            "x86_64" => "agx-win32-x64.exe".to_string(),
            "aarch64" => "agx-win32-arm64.exe".to_string(),
            other => format!("agx-win32-{other}.exe"),
        },
        "macos" => match std::env::consts::ARCH {
            "x86_64" => "agx-darwin-x64".to_string(),
            "aarch64" => "agx-darwin-arm64".to_string(),
            other => format!("agx-darwin-{other}"),
        },
        "linux" => match std::env::consts::ARCH {
            "x86_64" => "agx-linux-x64".to_string(),
            "aarch64" => "agx-linux-arm64".to_string(),
            other => format!("agx-linux-{other}"),
        },
        other => panic!("unsupported test platform: {other}"),
    }
}

fn standalone_manifest_json(version: &str, asset_name: &str, checksum: &str) -> String {
    let platform = match std::env::consts::OS {
        "windows" => "win32",
        "macos" => "darwin",
        "linux" => "linux",
        other => panic!("unsupported test platform: {other}"),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => panic!("unsupported test arch: {other}"),
    };
    format!(
        concat!(
            "{{\n",
            "  \"version\": \"{version}\",\n",
            "  \"assets\": [\n",
            "    {{\n",
            "      \"name\": \"{asset_name}\",\n",
            "      \"os\": \"{platform}\",\n",
            "      \"arch\": \"{arch}\",\n",
            "      \"sha256\": \"{checksum}\",\n",
            "      \"downloadUrl\": \"https://github.com/Drswith/agents-cli/releases/latest/download/{asset_name}\"\n",
            "    }}\n",
            "  ]\n",
            "}}\n"
        ),
        version = version,
        asset_name = asset_name,
        platform = platform,
        arch = arch,
        checksum = checksum,
    )
}
