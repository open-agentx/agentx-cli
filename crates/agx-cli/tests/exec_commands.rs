mod support;

use support::{TestWorkspace, run_agx, run_agx_with_env, stdout_json, stdout_json_lines};

#[test]
fn explicit_exec_returns_structured_process_result() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = run_agx(
        &workspace,
        &[
            "--json",
            "exec",
            "qoder",
            "--install",
            "never",
            "--",
            "--version",
        ],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["action"], "exec");
    assert_eq!(json["data"]["agent"]["name"], "qoder");
    assert_eq!(json["data"]["execution"]["installPolicy"], "never");
    assert_eq!(json["data"]["execution"]["exitCode"], 0);
    assert!(
        json["data"]["execution"]["stdout"]
            .as_str()
            .expect("stdout should be a string")
            .contains("agx 0.1.0")
    );
}

#[test]
fn shortcut_exec_uses_same_execution_path() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = run_agx(&workspace, &["qoder", "--", "--version"]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("agx 0.1.0"));
}

#[test]
fn exec_dry_run_reports_install_and_command_when_missing() {
    let workspace = TestWorkspace::new();
    let output = run_agx(
        &workspace,
        &[
            "--json",
            "--dry-run",
            "exec",
            "qoder",
            "--install",
            "if-missing",
            "--",
            "--version",
        ],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["execution"]["dryRun"], true);
    assert_eq!(json["data"]["execution"]["installPolicy"], "if-missing");
    assert_eq!(json["data"]["execution"]["installedBefore"], false);
    assert_eq!(json["data"]["execution"]["installedAfter"], false);
    assert_eq!(json["data"]["execution"]["command"][0], "qodercli");
    assert!(
        json["data"]["execution"]["message"]
            .as_str()
            .expect("message should be a string")
            .contains("would ensure Qoder CLI is installed")
    );
}

#[test]
fn exec_always_policy_runs_when_binary_is_already_present() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = run_agx(
        &workspace,
        &[
            "--json",
            "exec",
            "qoder",
            "--install",
            "always",
            "--",
            "--version",
        ],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["execution"]["installPolicy"], "always");
    assert_eq!(json["data"]["execution"]["installedBefore"], true);
    assert_eq!(json["data"]["execution"]["installedAfter"], true);
    assert_eq!(json["data"]["execution"]["exitCode"], 0);
}

#[test]
fn exec_ndjson_emits_single_result_event() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = run_agx(
        &workspace,
        &[
            "--output",
            "ndjson",
            "exec",
            "qoder",
            "--install",
            "never",
            "--",
            "--version",
        ],
    );

    assert!(output.status.success());
    let lines = stdout_json_lines(&output);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["type"], "result");
    assert_eq!(lines[0]["action"], "exec");
    assert_eq!(lines[0]["meta"]["mode"], "ndjson");
    assert_eq!(lines[0]["data"]["data"]["agent"]["name"], "qoder");
}

#[test]
fn exec_without_install_policy_returns_manual_action_required_when_missing() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "exec", "jcode", "--", "--version"]);

    assert_eq!(output.status.code(), Some(4));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "AGENT_NOT_INSTALLED");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should be a string")
            .contains("agx ensure jcode")
    );
    assert_eq!(
        json["data"]["execution"]["installGuidance"]["suggestedEnsureCommand"],
        "agx ensure jcode"
    );
    assert_eq!(
        json["data"]["execution"]["installGuidance"]["suggestedExecCommand"],
        "agx exec jcode --install if-missing -- --version"
    );
}

#[test]
fn exec_install_policy_alias_is_still_accepted() {
    let workspace = TestWorkspace::new();
    let output = run_agx(
        &workspace,
        &[
            "--json",
            "--dry-run",
            "exec",
            "qoder",
            "--install-policy",
            "if-missing",
            "--",
            "--version",
        ],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["execution"]["installPolicy"], "if-missing");
}

#[test]
fn shortcut_exec_rejects_structured_output_modes() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = run_agx(&workspace, &["--json", "qoder", "--", "--version"]);

    assert_eq!(output.status.code(), Some(2));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should be a string")
            .contains("Structured output is not supported")
    );
}

#[test]
fn shortcut_exec_non_interactive_returns_interaction_required_when_install_is_needed() {
    let workspace = TestWorkspace::new();
    let output = run_agx(
        &workspace,
        &["--non-interactive", "qoder", "--", "--version"],
    );

    assert_eq!(output.status.code(), Some(7));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("interactive installation is disabled"));
}

#[test]
fn shortcut_exec_installs_after_interactive_confirmation() {
    let workspace = TestWorkspace::new();
    let bin_dir = workspace.bin_dir().to_string_lossy().into_owned();

    let output = support::run_agx_with_stdin(
        &workspace,
        &["qoder", "--", "--version"],
        &[
            ("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1"),
            ("AGX_TEST_CREATE_BINARY_NAME", "qodercli"),
            ("AGX_TEST_CREATE_BINARY_DIR", &bin_dir),
        ],
        "y\n",
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("agx 0.1.0"));
}

#[test]
fn shortcut_exec_cancelled_install_returns_cancelled_error() {
    let workspace = TestWorkspace::new();

    let output = support::run_agx_with_stdin(&workspace, &["qoder", "--", "--version"], &[], "n\n");

    assert_eq!(output.status.code(), Some(11));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("cancelled"));
}

#[test]
fn shortcut_exec_assume_yes_installs_without_prompt() {
    let workspace = TestWorkspace::new();
    let bin_dir = workspace.bin_dir().to_string_lossy().into_owned();

    let output = support::run_agx_with_env(
        &workspace,
        &["--yes", "qoder", "--", "--version"],
        &[
            ("AGX_TEST_ALLOW_EXTERNAL_SUCCESS", "1"),
            ("AGX_TEST_CREATE_BINARY_NAME", "qodercli"),
            ("AGX_TEST_CREATE_BINARY_DIR", &bin_dir),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("agx 0.1.0"));
}

#[test]
fn shortcut_exec_install_failure_surfaces_install_error() {
    let workspace = TestWorkspace::new();

    let output = support::run_agx_with_stdin(&workspace, &["qoder", "--", "--version"], &[], "y\n");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("Failed to install Qoder CLI"));
}

#[test]
fn shortcut_exec_spawn_failure_surfaces_launch_error() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = run_agx_with_env(
        &workspace,
        &["qoder", "--", "--version"],
        &[("AGX_TEST_EXEC_MODE", "spawn-fail")],
    );

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("Failed to launch Qoder CLI"));
}

#[test]
fn exec_timeout_returns_timeout_error() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = support::run_agx_with_env(
        &workspace,
        &[
            "--json",
            "--timeout",
            "10ms",
            "exec",
            "qoder",
            "--install",
            "never",
            "--",
            "--version",
        ],
        &[("AGX_TEST_EXEC_MODE", "timeout")],
    );

    assert_eq!(output.status.code(), Some(10));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "TIMEOUT");
}

#[test]
fn exec_cancelled_returns_cancelled_error() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = support::run_agx_with_env(
        &workspace,
        &[
            "--json",
            "exec",
            "qoder",
            "--install",
            "never",
            "--",
            "--version",
        ],
        &[("AGX_TEST_EXEC_MODE", "cancelled")],
    );

    assert_eq!(output.status.code(), Some(11));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "CANCELLED");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should be a string")
            .contains("cancelled")
    );
}

#[test]
fn exec_spawn_failure_returns_invalid_argument() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = support::run_agx_with_env(
        &workspace,
        &[
            "--json",
            "exec",
            "qoder",
            "--install",
            "never",
            "--",
            "--version",
        ],
        &[("AGX_TEST_EXEC_MODE", "spawn-fail")],
    );

    assert_eq!(output.status.code(), Some(2));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
}
