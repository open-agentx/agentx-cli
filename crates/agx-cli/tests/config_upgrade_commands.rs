mod support;

use sha2::{Digest, Sha256};
use std::fs;

use support::{
    TestWorkspace, run_agx, run_agx_with_env, stdout_json, stdout_json_lines, stdout_text,
};

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
    assert_eq!(json["error"]["details"]["action"], "delete");
    assert_eq!(json["warnings"][0]["code"], "AVAILABLE_ACTIONS");
    assert_eq!(
        json["warnings"][0]["message"],
        "Available actions: get, set, reset"
    );
}

#[test]
fn upgrade_dry_run_without_latest_version_reports_check_unavailable_for_bun() {
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

    assert_eq!(output.status.code(), Some(6));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "NETWORK_ERROR");
    assert_eq!(json["data"]["installSource"], "bun");
    assert_eq!(json["data"]["status"], "check-unavailable");
}

#[test]
fn upgrade_dry_run_without_latest_version_reports_check_unavailable_for_npm() {
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

    assert_eq!(output.status.code(), Some(6));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "NETWORK_ERROR");
    assert_eq!(json["data"]["installSource"], "npm");
    assert_eq!(json["data"]["status"], "check-unavailable");
}

#[test]
fn upgrade_dry_run_reports_update_available_without_invoking_upgrade() {
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
        &["--json", "--dry-run", "upgrade"],
        &[("AGX_TEST_LATEST_VERSION", "0.2.0")],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["installSource"], "bun");
    assert_eq!(json["data"]["status"], "update-available");
    assert_eq!(json["warnings"][0]["code"], "DRY_RUN");
    assert!(json["data"]["command"].is_null());
    assert_eq!(
        json["warnings"]
            .as_array()
            .expect("warnings should exist")
            .len(),
        1
    );
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

#[test]
fn upgrade_check_reports_available_update_with_exit_code_one() {
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
        &["--json", "upgrade", "--check"],
        &[("AGX_TEST_LATEST_VERSION", "0.2.0")],
    );

    assert_eq!(output.status.code(), Some(1));
    let json = stdout_json(&output);
    assert_eq!(json["data"]["status"], "update-available");
    assert_eq!(json["data"]["channel"], "stable");
    assert_eq!(json["data"]["latestVersion"], "0.2.0");
    assert_eq!(json["data"]["canAutoUpdate"], true);
}

#[test]
fn upgrade_check_reports_check_unavailable_when_latest_version_cannot_be_resolved() {
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
        &["--json", "upgrade", "--check"],
        &[("AGX_TEST_LATEST_VERSION", "")],
    );

    assert_eq!(output.status.code(), Some(6));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "NETWORK_ERROR");
    assert_eq!(json["data"]["status"], "check-unavailable");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("Unable to determine the latest")
    );
}

#[test]
fn upgrade_check_unavailable_human_output_reports_resolution_failure() {
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
        &["upgrade", "--check"],
        &[("AGX_TEST_LATEST_VERSION", "")],
    );

    assert_eq!(output.status.code(), Some(6));
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Unable to determine the latest AGX version."));
    assert!(!stdout.contains("Failed to upgrade AGX"));
}

#[test]
fn upgrade_check_reports_up_to_date_when_versions_match() {
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

    let output = run_agx_with_env(
        &workspace,
        &["--json", "upgrade", "--check"],
        &[("AGX_TEST_LATEST_VERSION", "0.1.0")],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["status"], "up-to-date");
    assert_eq!(json["data"]["installSource"], "npm");
}

#[test]
fn upgrade_treats_lower_latest_version_as_stale_instead_of_downgrading() {
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

    let output = run_agx_with_env(
        &workspace,
        &["--json", "upgrade"],
        &[("AGX_TEST_LATEST_VERSION", "0.0.9")],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["status"], "up-to-date");
    assert_eq!(json["data"]["latestVersion"], "0.0.9");
    assert_eq!(json["warnings"][0]["code"], "STALE_LATEST_VERSION");
    assert!(
        json["warnings"][0]["message"]
            .as_str()
            .expect("warning should exist")
            .contains("will not downgrade")
    );
}

#[test]
fn upgrade_warns_when_selected_registry_lags_upstream_npm() {
    let workspace = TestWorkspace::new();
    workspace.write_config_bytes(
        br#"{
  "selfUpdateRegistry": "https://registry.npmmirror.com"
}
"#,
    );
    workspace.write_state_bytes(
        br#"{
  "installedAgents": {},
  "self": {
    "installSource": "npm"
  }
}
"#,
    );

    let output = run_agx_with_env(
        &workspace,
        &["--json", "upgrade"],
        &[
            ("AGX_TEST_LATEST_VERSION", "0.1.0"),
            ("AGX_TEST_UPSTREAM_LATEST_VERSION", "0.2.0"),
        ],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["status"], "up-to-date");
    assert_eq!(json["warnings"][0]["code"], "MIRROR_LAG");
    assert!(
        json["warnings"][0]["message"]
            .as_str()
            .expect("warning should exist")
            .contains("currently installs 0.1.0")
    );
}

#[test]
fn upgrade_check_uses_beta_channel_for_dist_tag_and_command() {
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

    let check_output = run_agx_with_env(
        &workspace,
        &["--json", "upgrade", "--check", "--channel", "beta"],
        &[("AGX_TEST_LATEST_VERSION", "0.3.0-beta.1")],
    );

    assert_eq!(check_output.status.code(), Some(1));
    let check_json = stdout_json(&check_output);
    assert_eq!(check_json["data"]["channel"], "beta");
    assert_eq!(check_json["data"]["latestVersion"], "0.3.0-beta.1");

    let dry_run_output = run_agx_with_env(
        &workspace,
        &["--json", "--dry-run", "upgrade", "--channel", "beta"],
        &[("AGX_TEST_LATEST_VERSION", "0.3.0-beta.1")],
    );
    assert!(dry_run_output.status.success());
    let dry_run_json = stdout_json(&dry_run_output);
    assert_eq!(dry_run_json["data"]["channel"], "beta");
    assert_eq!(dry_run_json["data"]["status"], "update-available");
    assert_eq!(dry_run_json["warnings"][0]["code"], "DRY_RUN");
}

#[test]
fn upgrade_human_output_prints_registry_lag_warning() {
    let workspace = TestWorkspace::new();
    workspace.write_config_bytes(
        br#"{
  "selfUpdateRegistry": "https://registry.npmmirror.com"
}
"#,
    );
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
        &["upgrade"],
        &[
            ("AGX_TEST_LATEST_VERSION", "0.1.0"),
            ("AGX_TEST_UPSTREAM_LATEST_VERSION", "0.2.0"),
        ],
    );

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("AGX is already up to date ("));
    assert!(stdout.contains("currently installs 0.1.0"));
}

#[test]
fn upgrade_human_output_reports_available_update_with_current_and_latest_versions() {
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
        &["upgrade", "--check"],
        &[("AGX_TEST_LATEST_VERSION", "0.2.0")],
    );

    assert_eq!(output.status.code(), Some(1));
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Update available for AGX CLI:"));
    assert!(stdout.contains("0.1.0 -> 0.2.0"));
    assert!(stdout.contains("(stable)."));
}

#[test]
fn upgrade_human_output_reports_successful_update_with_version_transition() {
    let workspace = TestWorkspace::new();
    let executable = workspace.install_fake_self_binary();
    let payload_path = workspace.root().join("agx-download.bin");
    let payload = fs::read(env!("CARGO_BIN_EXE_agx")).expect("test binary should exist");
    let checksum = format!("{:x}", Sha256::digest(&payload));
    fs::write(&payload_path, &payload).expect("payload should be written");
    let manifest_path = workspace.root().join("manifest.json");
    fs::write(
        &manifest_path,
        standalone_manifest_json("0.2.0", &standalone_asset_name(), &checksum, None),
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
        &["upgrade"],
        &[
            (
                "AGX_TEST_SELF_EXECUTABLE_PATH",
                executable.to_string_lossy().as_ref(),
            ),
            (
                "AGX_TEST_STANDALONE_MANIFEST_PATH",
                manifest_path.to_string_lossy().as_ref(),
            ),
            (
                "AGX_TEST_STANDALONE_DOWNLOAD_PATH",
                payload_path.to_string_lossy().as_ref(),
            ),
            ("AGX_TEST_SKIP_STANDALONE_VERIFY", "1"),
        ],
    );

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Upgrading AGX CLI... (0.1.0 -> 0.2.0)"));
    assert!(stdout.contains("AGX CLI upgraded successfully."));
}

#[test]
fn upgrade_ndjson_emits_single_result_event() {
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
        &["--output", "ndjson", "--dry-run", "upgrade"],
        &[("AGX_TEST_LATEST_VERSION", "0.2.0")],
    );

    assert!(output.status.success());
    let lines = stdout_json_lines(&output);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["type"], "result");
    assert_eq!(lines[0]["action"], "upgrade");
    assert_eq!(lines[0]["meta"]["mode"], "ndjson");
    assert_eq!(lines[0]["data"]["data"]["installSource"], "bun");
}

#[test]
fn upgrade_failure_surfaces_bun_recovery_hint() {
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
        &["--json", "upgrade"],
        &[
            ("AGX_TEST_LATEST_VERSION", "0.2.0"),
            ("AGX_TEST_UPGRADE_FAILURE", "bun"),
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "UPGRADE_FAILED");
    assert_eq!(json["error"]["details"]["kind"], "unknown");
    assert_eq!(json["data"]["status"], "manual-required");
    assert_eq!(
        json["data"]["recoveryHint"],
        "bun add -g @open-agentx/agentx-cli@latest"
    );
    assert_eq!(json["warnings"][0]["code"], "MANUAL_RECOVERY");
    assert_eq!(
        json["warnings"][0]["message"],
        "Manual recovery: bun add -g @open-agentx/agentx-cli@latest"
    );
}

#[test]
fn upgrade_failure_surfaces_npm_recovery_hint() {
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

    let output = run_agx_with_env(
        &workspace,
        &["--json", "upgrade"],
        &[
            ("AGX_TEST_LATEST_VERSION", "0.2.0"),
            ("AGX_TEST_UPGRADE_FAILURE", "npm"),
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "UPGRADE_FAILED");
    assert_eq!(json["error"]["details"]["kind"], "unknown");
    assert_eq!(json["data"]["status"], "manual-required");
    assert_eq!(
        json["data"]["recoveryHint"],
        "npm install -g @open-agentx/agentx-cli@latest"
    );
    assert_eq!(json["warnings"][0]["code"], "MANUAL_RECOVERY");
}

#[test]
fn upgrade_failure_surfaces_lock_retry_hint() {
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

    let output = run_agx_with_env(
        &workspace,
        &["--json", "upgrade"],
        &[
            ("AGX_TEST_LATEST_VERSION", "0.2.0"),
            ("AGX_TEST_UPGRADE_FAILURE", "locked"),
        ],
    );

    assert_eq!(output.status.code(), Some(9));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "RESOURCE_LOCKED");
    assert_eq!(json["error"]["details"]["kind"], "locked");
    assert_eq!(json["data"]["status"], "manual-required");
    assert_eq!(
        json["data"]["recoveryHint"],
        "npm install -g @open-agentx/agentx-cli@latest"
    );
    assert_eq!(json["warnings"][0]["code"], "MANUAL_RECOVERY");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("already running")
    );
}

#[test]
fn upgrade_standalone_dry_run_returns_download_plan() {
    let workspace = TestWorkspace::new();
    let executable = workspace.install_fake_self_binary();
    let manifest_path = workspace.root().join("manifest.json");
    fs::write(
        &manifest_path,
        standalone_manifest_json("0.2.0", &standalone_asset_name(), "placeholder", None),
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
        &["--json", "--dry-run", "upgrade"],
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
    let json = stdout_json(&output);
    assert_eq!(json["data"]["installSource"], "standalone");
    assert_eq!(json["data"]["status"], "update-available");
    assert_eq!(json["warnings"][0]["code"], "DRY_RUN");
    assert!(json["data"]["recoveryHint"].is_null());
    assert!(json["data"]["command"].is_null());
}

#[test]
fn upgrade_failure_human_output_surfaces_reason_and_next_step() {
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
        &["upgrade"],
        &[
            ("AGX_TEST_LATEST_VERSION", "0.2.0"),
            ("AGX_TEST_UPGRADE_FAILURE", "bun"),
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    let stdout = stdout_text(&output);
    assert!(stdout.contains("Upgrading AGX CLI... (0.1.0 -> 0.2.0)"));
    assert!(stdout.contains("Failed to upgrade AGX CLI."));
    assert!(stdout.contains("Reason: Failed to update @open-agentx/agentx-cli through Bun."));
    assert!(stdout.contains("Next step: bun add -g @open-agentx/agentx-cli@latest"));
    assert!(
        !stdout
            .contains("Reason: Failed to update @open-agentx/agentx-cli through Bun. Next step:")
    );
}

#[test]
fn upgrade_check_uses_standalone_manifest_version() {
    let workspace = TestWorkspace::new();
    let executable = workspace.install_fake_self_binary();
    let manifest_path = workspace.root().join("manifest.json");
    fs::write(
        &manifest_path,
        standalone_manifest_json("0.2.0", &standalone_asset_name(), "placeholder", None),
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
        &["--json", "upgrade", "--check"],
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

    assert_eq!(output.status.code(), Some(1));
    let json = stdout_json(&output);
    assert_eq!(json["data"]["status"], "update-available");
    assert_eq!(json["data"]["installSource"], "standalone");
    assert_eq!(json["data"]["latestVersion"], "0.2.0");
}

#[test]
fn upgrade_standalone_succeeds_with_checksum_verified_payload() {
    let workspace = TestWorkspace::new();
    let executable = workspace.install_fake_self_binary();
    let payload_path = workspace.root().join("agx-download.bin");
    let payload = fs::read(env!("CARGO_BIN_EXE_agx")).expect("test binary should exist");
    let checksum = format!("{:x}", Sha256::digest(&payload));
    fs::write(&payload_path, &payload).expect("payload should be written");
    let manifest_path = workspace.root().join("manifest.json");
    fs::write(
        &manifest_path,
        standalone_manifest_json("0.2.0", &standalone_asset_name(), &checksum, None),
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
        &["--json", "upgrade"],
        &[
            (
                "AGX_TEST_SELF_EXECUTABLE_PATH",
                executable.to_string_lossy().as_ref(),
            ),
            (
                "AGX_TEST_STANDALONE_MANIFEST_PATH",
                manifest_path.to_string_lossy().as_ref(),
            ),
            (
                "AGX_TEST_STANDALONE_DOWNLOAD_PATH",
                payload_path.to_string_lossy().as_ref(),
            ),
            ("AGX_TEST_SKIP_STANDALONE_VERIFY", "1"),
        ],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["status"], "updated");
    assert_eq!(json["data"]["installSource"], "standalone");
    assert!(json["data"]["verifiedVersion"].is_null());
    let replaced = fs::read(&executable).expect("executable should still exist");
    assert_eq!(replaced, payload);
}

#[test]
fn upgrade_standalone_rejects_checksum_mismatch() {
    let workspace = TestWorkspace::new();
    let executable = workspace.install_fake_self_binary();
    let payload_path = workspace.root().join("agx-download.bin");
    let payload = fs::read(env!("CARGO_BIN_EXE_agx")).expect("test binary should exist");
    fs::write(&payload_path, &payload).expect("payload should be written");
    let manifest_path = workspace.root().join("manifest.json");
    fs::write(
        &manifest_path,
        standalone_manifest_json("0.2.0", &standalone_asset_name(), "deadbeef", None),
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
        &["--json", "upgrade"],
        &[
            (
                "AGX_TEST_SELF_EXECUTABLE_PATH",
                executable.to_string_lossy().as_ref(),
            ),
            (
                "AGX_TEST_STANDALONE_MANIFEST_PATH",
                manifest_path.to_string_lossy().as_ref(),
            ),
            (
                "AGX_TEST_STANDALONE_DOWNLOAD_PATH",
                payload_path.to_string_lossy().as_ref(),
            ),
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "UPGRADE_FAILED");
    assert_eq!(json["data"]["status"], "manual-required");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("Checksum mismatch")
    );
    assert!(
        json["data"]["recoveryHint"]
            .as_str()
            .expect("recoveryHint should exist")
            .contains(&standalone_asset_name())
    );
}

#[test]
fn upgrade_standalone_reports_missing_asset_as_manual_action() {
    let workspace = TestWorkspace::new();
    let executable = workspace.install_fake_self_binary();
    let manifest_path = workspace.root().join("manifest.json");
    fs::write(
        &manifest_path,
        standalone_manifest_json("0.2.0", "agx-linux-arm64", "placeholder", None),
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
        &["--json", "upgrade"],
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

    assert_eq!(output.status.code(), Some(8));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "MANUAL_ACTION_REQUIRED");
    assert_eq!(json["data"]["status"], "manual-required");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("No standalone AGX release asset")
    );
}

#[test]
fn upgrade_check_reads_cached_latest_version_by_default() {
    let workspace = TestWorkspace::new();
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
            "    \"npm:https://registry.npmjs.org:@open-agentx/agentx-cli:latest\": {\n",
            "      \"body\": \"{\\\"version\\\":\\\"0.2.0\\\"}\",\n",
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
  "installedAgents": {},
  "self": {
    "installSource": "npm"
  }
}
"#,
    );

    let output = run_agx(&workspace, &["--json", "upgrade", "--check"]);

    assert_eq!(output.status.code(), Some(1));
    let json = stdout_json(&output);
    assert_eq!(json["data"]["latestVersion"], "0.2.0");
    assert_eq!(json["data"]["status"], "update-available");
}

#[test]
fn upgrade_check_no_cache_ignores_stale_cached_latest_version() {
    let workspace = TestWorkspace::new();
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
            "    \"npm:https://registry.npmjs.org:@open-agentx/agentx-cli:latest\": {\n",
            "      \"body\": \"{\\\"version\\\":\\\"0.2.0\\\"}\",\n",
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
  "installedAgents": {},
  "self": {
    "installSource": "npm"
  }
}
"#,
    );

    let output = run_agx_with_env(
        &workspace,
        &["--json", "--no-cache", "upgrade", "--check"],
        &[("AGX_TEST_LATEST_VERSION", "0.1.0")],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["latestVersion"], "0.1.0");
    assert_eq!(json["data"]["status"], "up-to-date");
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

fn standalone_manifest_json(
    version: &str,
    asset_name: &str,
    checksum: &str,
    download_url: Option<&str>,
) -> String {
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
    let download_url = download_url.map_or_else(
        || {
            format!(
                "https://github.com/open-agentx/agentx-cli/releases/latest/download/{asset_name}"
            )
        },
        ToString::to_string,
    );
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
            "      \"downloadUrl\": \"{download_url}\"\n",
            "    }}\n",
            "  ]\n",
            "}}\n"
        ),
        version = version,
        asset_name = asset_name,
        platform = platform,
        arch = arch,
        checksum = checksum,
        download_url = download_url,
    )
}
