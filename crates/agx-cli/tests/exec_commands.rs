mod support;

use support::{TestWorkspace, run_agx, stdout_json};

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
            "--install-policy",
            "never",
            "--",
            "--version",
        ],
    );

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["action"], "exec");
    assert_eq!(json["data"]["agent"]["name"], "qoder");
    assert_eq!(json["data"]["installPolicy"], "never");
    assert_eq!(json["data"]["exitCode"], 0);
    assert!(
        json["data"]["stdout"]
            .as_str()
            .expect("stdout should be a string")
            .contains("agx 0.1.0")
    );
}

#[test]
fn shortcut_exec_uses_same_execution_path() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

    let output = run_agx(&workspace, &["--json", "qoder", "--", "--version"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["action"], "exec");
    assert_eq!(json["data"]["agent"]["name"], "qoder");
    assert_eq!(json["data"]["args"][0], "--version");
}

#[test]
fn exec_without_install_policy_returns_manual_action_required_when_missing() {
    let workspace = TestWorkspace::new();
    let output = run_agx(
        &workspace,
        &[
            "--json",
            "exec",
            "jcode",
            "--install-policy",
            "never",
            "--",
            "--version",
        ],
    );

    assert_eq!(output.status.code(), Some(8));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "MANUAL_ACTION_REQUIRED");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should be a string")
            .contains("agx ensure jcode")
    );
}
