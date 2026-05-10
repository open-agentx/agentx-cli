mod support;

use std::fs;

use support::{TestWorkspace, run_agx, stdout_json, stdout_text};

#[test]
fn list_marks_installed_agents_when_binary_is_present() {
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

    let output = run_agx(&workspace, &["--json", "list"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let agents = json["data"]["agents"]
        .as_array()
        .expect("agents should be an array");
    let qoder = agents
        .iter()
        .find(|agent| agent["name"] == "qoder")
        .expect("qoder should exist");

    assert_eq!(qoder["installed"], true);
    assert_eq!(qoder["binaryName"], "qodercli");
    assert_eq!(qoder["installedVersion"], "0.1.0");
    assert_eq!(qoder["lifecycle"], "managed");
    assert_eq!(qoder["sourceLabel"], "managed via bun (@qoder-ai/qodercli)");
    assert_eq!(qoder["updateLabel"], "managed update");
}

#[test]
fn list_marks_missing_agents_as_uninstalled() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "list"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let agents = json["data"]["agents"]
        .as_array()
        .expect("agents should be an array");
    let qoder = agents
        .iter()
        .find(|agent| agent["name"] == "qoder")
        .expect("qoder should exist");

    assert_eq!(qoder["installed"], false);
    assert_eq!(qoder["sourceLabel"], "untracked");
    assert_eq!(qoder["updateLabel"], "command update");
}

#[test]
fn list_marks_untracked_installed_agents_with_command_update_and_no_version() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("amp");

    let output = run_agx(&workspace, &["--json", "list"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let agents = json["data"]["agents"]
        .as_array()
        .expect("agents should be an array");
    let amp = agents
        .iter()
        .find(|agent| agent["name"] == "amp")
        .expect("amp should exist");

    assert_eq!(amp["installed"], true);
    assert!(amp["installedVersion"].is_null());
    assert_eq!(amp["sourceLabel"], "detected in PATH");
    assert_eq!(amp["updateLabel"], "command update");
}

#[test]
fn list_human_output_shows_unknown_version_for_installed_agents_without_probe_result() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("amp");

    let output = run_agx(&workspace, &["list"]);

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Amp"));
    assert!(stdout.contains("installed  unknown version"));
    assert!(stdout.contains("command update"));
    assert!(stdout.contains("detected in PATH"));
}

#[test]
fn list_human_output_marks_missing_agents_as_not_installed() {
    let workspace = TestWorkspace::new();

    let output = run_agx(&workspace, &["list"]);

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Qoder CLI"));
    assert!(stdout.contains("not installed"));
    assert!(!stdout.contains("not installed  command update"));
    assert!(!stdout.contains("not installed  manual update"));
    assert!(!stdout.contains("not installed  untracked"));
}

#[test]
fn list_prefers_npm_cmd_for_managed_versions_on_windows_style_paths() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("deepseek");
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {
    "deepseek": {
      "agentName": "deepseek",
      "installType": "npm",
      "packageName": "deepseek-tui",
      "packageTargetKind": "package"
    }
  },
  "self": {}
}
"#,
    );
    fs::create_dir_all(workspace.bin_dir()).expect("bin dir should exist");
    if cfg!(windows) {
        fs::write(
            workspace.bin_dir().join("npm"),
            "this shim should not be executed by managed npm detection\r\n",
        )
        .expect("shim file should be written");
        fs::write(
            workspace.bin_dir().join("npm.cmd"),
            "@echo off\r\n\
echo {\"dependencies\":{\"deepseek-tui\":{\"version\":\"0.8.24\"}}}\r\n",
        )
        .expect("npm.cmd should be written");
    } else {
        fs::write(
            workspace.bin_dir().join("npm"),
            "#!/bin/sh\nprintf '%s\\n' '{\"dependencies\":{\"deepseek-tui\":{\"version\":\"0.8.24\"}}}'\n",
        )
        .expect("npm shim should be written");
    }

    let output = run_agx(&workspace, &["--json", "list"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let deepseek = json["data"]["agents"]
        .as_array()
        .expect("agents should be an array")
        .iter()
        .find(|agent| agent["name"] == "deepseek")
        .expect("deepseek should exist");

    assert_eq!(deepseek["installed"], true);
    assert_eq!(deepseek["installedVersion"], "0.8.24");
    assert_eq!(deepseek["sourceLabel"], "managed via npm (deepseek-tui)");
}

#[test]
fn info_resolves_aliases_to_canonical_agent() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "info", "qodercli"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["agent"]["name"], "qoder");
    assert_eq!(json["data"]["agent"]["displayName"], "Qoder CLI");
    assert!(json["data"]["inspection"]["updateLabel"].is_null());
    assert!(json["data"]["inspection"]["sourceLabel"].is_null());
    assert_eq!(
        json["data"]["agent"]["selfUpdateCommands"][0],
        "qodercli update"
    );
}

#[test]
fn info_unknown_agent_returns_agent_not_found() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "info", "missing-agent"]);

    assert_eq!(output.status.code(), Some(3));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "AGENT_NOT_FOUND");
    assert_eq!(json["error"]["details"]["input"], "missing-agent");
    assert_eq!(json["error"]["details"]["input"], "missing-agent");
}

#[test]
fn inspect_exposes_install_methods_and_self_update_metadata() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "inspect", "claude"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let methods = json["data"]["capabilities"]["installMethods"]
        .as_array()
        .expect("install methods should be an array");
    let commands: Vec<_> = methods
        .iter()
        .filter_map(|method| method["command"].as_str())
        .collect();

    assert!(commands.contains(&"bun add -g @anthropic-ai/claude-code"));
    assert!(commands.contains(&"npm install -g @anthropic-ai/claude-code"));
    assert_eq!(json["data"]["capabilities"]["canAutoInstall"], true);
    assert_eq!(json["data"]["capabilities"]["canRun"], false);
    assert_eq!(
        json["data"]["capabilities"]["selfUpdateCommands"][0],
        "claude update"
    );
}

#[test]
fn inspect_human_output_shows_latest_version_for_installed_agent() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");
    fs::create_dir_all(
        workspace
            .cache_file()
            .parent()
            .expect("cache parent should exist"),
    )
    .expect("cache directory should be created");
    fs::write(
        workspace.cache_file(),
        concat!(
            "{\n",
            "  \"entries\": {\n",
            "    \"npm:https://registry.npmjs.org:@qoder-ai/qodercli:latest\": {\n",
            "      \"body\": \"{\\\"version\\\":\\\"9.9.9\\\"}\",\n",
            "      \"expiresAt\": 4102444800000,\n",
            "      \"fetchedAt\": 4102441200000\n",
            "    }\n",
            "  }\n",
            "}\n"
        ),
    )
    .expect("cache file should be written");
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

    let output = run_agx(&workspace, &["inspect", "qoder"]);

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Latest:       9.9.9"));
}

#[test]
fn inspect_unknown_agent_returns_agent_not_found() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "inspect", "missing-agent"]);

    assert_eq!(output.status.code(), Some(3));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "AGENT_NOT_FOUND");
}

#[test]
fn inspect_manual_only_agent_reports_no_managed_install_methods() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "inspect", "jcode"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["agent"]["name"], "jcode");
    assert_eq!(
        json["data"]["capabilities"]["installMethods"]
            .as_array()
            .expect("install methods should be an array")
            .len(),
        0
    );
}

#[test]
fn resolve_returns_install_guidance_for_missing_binary() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "resolve", "qoder"]);

    assert_eq!(output.status.code(), Some(4));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "AGENT_NOT_INSTALLED");
    assert_eq!(json["data"]["resolution"]["installed"], false);
    assert_eq!(
        json["data"]["resolution"]["installGuidance"]["suggestedAction"],
        "ensure-agent-installed"
    );
    assert_eq!(
        json["data"]["resolution"]["installGuidance"]["suggestedEnsureCommand"],
        "agx ensure qoder"
    );
    assert_eq!(json["data"]["resolution"]["installSource"], "not-installed");
    assert_eq!(json["data"]["resolution"]["lifecycle"], "unmanaged");
    assert_eq!(json["data"]["resolution"]["sourceLabel"], "not installed");
    assert_eq!(
        json["error"]["details"]["suggestedEnsureCommand"],
        "agx ensure qoder"
    );
    assert_eq!(
        json["error"]["details"]["suggestedAction"],
        "ensure-agent-installed"
    );
    assert!(
        json["data"]["resolution"]["suggestedLaunchCommand"]
            .as_array()
            .expect("launch command should be an array")
            .is_empty()
    );
}

#[test]
fn resolve_unknown_agent_returns_agent_not_found() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "resolve", "missing-agent"]);

    assert_eq!(output.status.code(), Some(3));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "AGENT_NOT_FOUND");
    assert_eq!(json["error"]["details"]["input"], "missing-agent");
}

#[test]
fn resolve_manual_only_agent_returns_empty_install_methods() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "resolve", "jcode"]);

    assert_eq!(output.status.code(), Some(4));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "AGENT_NOT_INSTALLED");
    assert_eq!(json["data"]["resolution"]["installed"], false);
    assert_eq!(
        json["data"]["resolution"]["installGuidance"]["suggestedEnsureCommand"],
        "agx ensure jcode"
    );
    assert_eq!(
        json["data"]["resolution"]["installGuidance"]["installMethods"]
            .as_array()
            .expect("install methods should be an array")
            .len(),
        0
    );
}

#[test]
fn resolve_returns_binary_path_for_installed_agent() {
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

    let output = run_agx(&workspace, &["--json", "resolve", "qoder"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["resolution"]["installed"], true);
    assert_eq!(json["data"]["resolution"]["installSource"], "bun");
    assert_eq!(
        json["data"]["resolution"]["suggestedLaunchCommand"][0],
        json["data"]["resolution"]["binaryPath"]
    );
    assert!(
        json["data"]["resolution"]["binaryPath"]
            .as_str()
            .expect("binary path should exist")
            .contains("qodercli")
    );
    assert!(json["data"]["resolution"]["installGuidance"].is_null());
    assert_eq!(json["data"]["resolution"]["installedVersion"], "0.1.0");
}

#[test]
fn resolve_uses_detected_in_path_for_untracked_installed_agent() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = run_agx(&workspace, &["--json", "resolve", "qoder"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["resolution"]["installed"], true);
    assert_eq!(
        json["data"]["resolution"]["installSource"],
        "detected-in-path"
    );
    assert_eq!(json["data"]["resolution"]["lifecycle"], "unmanaged");
    assert_eq!(
        json["data"]["resolution"]["sourceLabel"],
        "detected in PATH"
    );
}

#[test]
fn resolve_human_output_prints_ensure_guidance_for_missing_agent_binary() {
    let workspace = TestWorkspace::new();

    let output = run_agx(&workspace, &["resolve", "qoder"]);

    assert_eq!(output.status.code(), Some(4));
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Qoder CLI is not installed."));
    assert!(stdout.contains("Try: agx ensure qoder"));
    assert!(stdout.contains("Install: [bun] bun add -g @qoder-ai/qodercli"));
    assert!(stdout.contains("Install: [npm] npm install -g @qoder-ai/qodercli"));
}

#[test]
fn resolve_human_output_prints_binary_details_for_installed_agent() {
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

    let output = run_agx(&workspace, &["resolve", "qoder"]);

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Qoder CLI"));
    assert!(stdout.contains("Name:          qoder"));
    assert!(stdout.contains("Binary:        qodercli"));
    assert!(stdout.contains("Path:"));
    assert!(stdout.contains("Source:        managed via bun (@qoder-ai/qodercli)"));
    assert!(stdout.contains("Lifecycle:     managed"));
    assert!(stdout.contains("Install Type:  bun"));
    assert!(stdout.contains("Version:       0.1.0"));
    assert!(stdout.contains("Launch:"));
}

#[test]
fn info_human_output_includes_install_methods_and_source_details() {
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

    let output = run_agx(&workspace, &["info", "qoder"]);
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");

    assert!(output.status.success());
    assert!(stdout.contains("Qoder CLI"));
    assert!(stdout.contains("managed via bun (@qoder-ai/qodercli)"));
    assert!(stdout.contains("Install Methods"));
    assert!(stdout.contains("[bun]"));
    assert!(stdout.contains("bun add -g @qoder-ai/qodercli"));
}

#[test]
fn info_human_output_lists_aliases_and_self_update_commands() {
    let workspace = TestWorkspace::new();

    let output = run_agx(&workspace, &["info", "qoder"]);

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Aliases:"));
    assert!(stdout.contains("qodercli"));
    assert!(stdout.contains("Update:"));
    assert!(stdout.contains("qodercli update"));
    assert!(!stdout.contains("Lifecycle:"));
}

#[test]
fn inspect_human_output_includes_capabilities_and_update_mode() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("claude");

    let output = run_agx(&workspace, &["inspect", "claude"]);
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");

    assert!(output.status.success());
    assert!(stdout.contains("Capabilities"));
    assert!(stdout.contains("Update Mode:"));
    assert!(stdout.contains("command update"));
    assert!(stdout.contains("auto-install:"));
    assert!(stdout.contains("self-update:"));
}

#[test]
fn inspect_manual_only_agent_marks_auto_install_as_unavailable() {
    let workspace = TestWorkspace::new();

    let output = run_agx(&workspace, &["--json", "inspect", "jcode"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["capabilities"]["canAutoInstall"], false);
    assert_eq!(json["data"]["capabilities"]["canAutoUninstall"], false);
    assert_eq!(json["data"]["capabilities"]["canRun"], false);
}

#[test]
fn list_reads_cached_latest_version_metadata() {
    let workspace = TestWorkspace::new();
    workspace.write_config_bytes(
        br#"{
  "selfUpdateRegistry": "http://127.0.0.1:9"
}
"#,
    );
    fs::create_dir_all(
        workspace
            .cache_file()
            .parent()
            .expect("cache parent should exist"),
    )
    .expect("cache directory should be created");
    fs::write(
        workspace.cache_file(),
        concat!(
            "{\n",
            "  \"entries\": {\n",
            "    \"npm:http://127.0.0.1:9:@qoder-ai/qodercli:latest\": {\n",
            "      \"body\": \"{\\\"version\\\":\\\"9.9.9\\\"}\",\n",
            "      \"expiresAt\": 4102444800000,\n",
            "      \"fetchedAt\": 4102441200000\n",
            "    }\n",
            "  }\n",
            "}\n"
        ),
    )
    .expect("cache file should be written");

    let output = run_agx(&workspace, &["--json", "list"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let agents = json["data"]["agents"]
        .as_array()
        .expect("agents should be an array");
    let qoder = agents
        .iter()
        .find(|agent| agent["name"] == "qoder")
        .expect("qoder should exist");
    assert_eq!(qoder["latestVersion"], "9.9.9");
    assert_eq!(json["meta"]["source"], "cache");
    assert_eq!(json["meta"]["fetchedAt"], "2099-12-31T23:00:00.000Z");
    assert_eq!(json["meta"]["staleAfter"], "2100-01-01T00:00:00.000Z");
}

#[test]
fn list_human_output_uses_column_style_layout() {
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

    let output = run_agx(&workspace, &["list"]);

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("AI Agents:"));
    assert!(stdout.contains("Qoder CLI"));
    assert!(stdout.contains("installed"));
    assert!(stdout.contains("0.1.0"));
    assert!(stdout.contains("managed update"));
    assert!(stdout.contains("managed via bun (@qoder-ai/qodercli)"));
}

#[test]
fn info_exposes_reasonix_catalog_entry() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "info", "reasonix"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["agent"]["name"], "reasonix");
    assert_eq!(json["data"]["agent"]["displayName"], "Reasonix");
    assert_eq!(json["data"]["agent"]["aliases"][0], "deepseek-reasonix");
    assert_eq!(json["data"]["agent"]["packageName"], "reasonix");
    assert_eq!(
        json["data"]["agent"]["selfUpdateCommands"][0],
        "reasonix update"
    );
}
