mod support;

use support::{TestWorkspace, run_agx, stdout_json};

#[test]
fn list_marks_installed_agents_when_binary_is_present() {
    let workspace = TestWorkspace::new();
    workspace.install_fake_agent_binary("qodercli");

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
    assert_eq!(qoder["updateLabel"], "manual");
}

#[test]
fn info_resolves_aliases_to_canonical_agent() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "info", "qodercli"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["agent"]["name"], "qoder");
    assert_eq!(json["data"]["agent"]["displayName"], "Qoder CLI");
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
    assert_eq!(
        json["data"]["capabilities"]["selfUpdateCommands"][0],
        "claude update"
    );
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

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["resolution"]["installed"], false);
    assert_eq!(
        json["data"]["resolution"]["installGuidance"]["suggestedEnsureCommand"],
        "agx ensure qoder"
    );
}

#[test]
fn resolve_unknown_agent_returns_agent_not_found() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "resolve", "missing-agent"]);

    assert_eq!(output.status.code(), Some(3));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "AGENT_NOT_FOUND");
}

#[test]
fn resolve_manual_only_agent_returns_empty_install_methods() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "resolve", "jcode"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
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

    let output = run_agx(&workspace, &["--json", "resolve", "qoder"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    assert_eq!(json["data"]["resolution"]["installed"], true);
    assert!(
        json["data"]["resolution"]["binaryPath"]
            .as_str()
            .expect("binary path should exist")
            .contains("qodercli")
    );
    assert!(json["data"]["resolution"]["installGuidance"].is_null());
}
