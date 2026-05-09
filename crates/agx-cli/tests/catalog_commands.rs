mod support;

use support::{TestWorkspace, run_agx, stdout_json, stdout_json_lines, stdout_text};

#[test]
fn commands_json_includes_core_lifecycle_commands() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "commands"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let commands = json["data"]["commands"]
        .as_array()
        .expect("commands should be an array");

    let names: Vec<_> = commands
        .iter()
        .filter_map(|command| command["name"].as_str())
        .collect();

    assert!(names.contains(&"exec"));
    assert!(names.contains(&"update"));
    assert!(names.contains(&"upgrade"));
    assert!(names.contains(&"doctor"));
}

#[test]
fn commands_human_output_shows_flags_and_schema_refs() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["commands"]);

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("AGX Commands"));
    assert!(stdout.contains("commands"));
    assert!(stdout.contains("[--json, --output, --quiet, --color, --log-level, --timeout]"));
    assert!(stdout.contains("#/commands/commands"));
    assert!(!stdout.contains("Run `agx commands --json`"));
}

#[test]
fn commands_json_describes_install_exec_and_inspect_flags() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "commands"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let commands = json["data"]["commands"]
        .as_array()
        .expect("commands should be an array");

    let install = commands
        .iter()
        .find(|command| command["name"] == "install")
        .expect("install command should exist");
    let exec = commands
        .iter()
        .find(|command| command["name"] == "exec")
        .expect("exec command should exist");
    let inspect = commands
        .iter()
        .find(|command| command["name"] == "inspect")
        .expect("inspect command should exist");

    let install_flags: Vec<_> = install["flags"]
        .as_array()
        .expect("install flags should be an array")
        .iter()
        .filter_map(|flag| flag.as_str())
        .collect();
    let inspect_flags: Vec<_> = inspect["flags"]
        .as_array()
        .expect("inspect flags should be an array")
        .iter()
        .filter_map(|flag| flag.as_str())
        .collect();
    let exec_flags: Vec<_> = exec["flags"]
        .as_array()
        .expect("exec flags should be an array")
        .iter()
        .filter_map(|flag| flag.as_str())
        .collect();

    assert!(install_flags.contains(&"--yes"));
    assert!(install_flags.contains(&"--dry-run"));
    assert!(!install_flags.contains(&"--channel"));
    assert!(!install_flags.contains(&"--check"));
    assert_eq!(install["summary"], "Install one or more agents");
    assert!(exec_flags.contains(&"--install"));
    assert!(exec_flags.contains(&"--non-interactive"));
    assert!(!exec_flags.contains(&"--install-policy"));
    assert!(!exec_flags.contains(&"--json"));
    assert!(!exec_flags.contains(&"--timeout"));
    assert!(inspect_flags.contains(&"--refresh"));
    assert!(inspect_flags.contains(&"--no-cache"));
}

#[test]
fn commands_json_includes_schema_refs_and_stability() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "commands"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let commands = json["data"]["commands"]
        .as_array()
        .expect("commands should be an array");

    let update = commands
        .iter()
        .find(|command| command["name"] == "update")
        .expect("update command should exist");
    let schema = commands
        .iter()
        .find(|command| command["name"] == "schema")
        .expect("schema command should exist");
    let upgrade = commands
        .iter()
        .find(|command| command["name"] == "upgrade")
        .expect("upgrade command should exist");

    assert_eq!(update["outputSchemaRef"], "#/commands/update");
    assert_eq!(update["stability"], "stable");
    assert_eq!(schema["summary"], "Return structured output schemas");
    assert!(
        upgrade["flags"]
            .as_array()
            .expect("upgrade flags should be an array")
            .iter()
            .any(|flag| flag == "--channel")
    );
    assert!(
        upgrade["flags"]
            .as_array()
            .expect("upgrade flags should be an array")
            .iter()
            .any(|flag| flag == "--check")
    );
    assert!(
        upgrade["flags"]
            .as_array()
            .expect("upgrade flags should be an array")
            .iter()
            .any(|flag| flag == "--refresh")
    );
    assert!(
        upgrade["flags"]
            .as_array()
            .expect("upgrade flags should be an array")
            .iter()
            .any(|flag| flag == "--no-cache")
    );
    assert!(
        upgrade["flags"]
            .as_array()
            .expect("upgrade flags should be an array")
            .iter()
            .any(|flag| flag == "--idempotency-key")
    );
    assert!(
        !upgrade["flags"]
            .as_array()
            .expect("upgrade flags should be an array")
            .iter()
            .any(|flag| flag == "--yes")
    );
}

#[test]
fn schema_human_output_shows_descriptions_only() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["schema"]);

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("AGX Schemas"));
    assert!(stdout.contains("commands"));
    assert!(stdout.contains("Stable command catalog"));
    assert!(!stdout.contains("Run `agx schema --json`"));
}

#[test]
fn schema_json_includes_lifecycle_and_workflow_surfaces() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "schema"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let schemas = json["data"]["commands"]
        .as_array()
        .expect("schemas should be an array");

    let names: Vec<_> = schemas
        .iter()
        .filter_map(|schema| schema["name"].as_str())
        .collect();

    assert!(names.contains(&"exec"));
    assert!(names.contains(&"update"));
    assert!(names.contains(&"upgrade"));
}

#[test]
fn schema_json_filters_to_a_specific_command() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "schema", "inspect"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let schemas = json["data"]["commands"]
        .as_array()
        .expect("schemas should be an array");

    assert_eq!(schemas.len(), 1);
    assert_eq!(schemas[0]["name"], "inspect");
    assert_eq!(schemas[0]["ndjsonEventSchema"]["type"], "object");
    let properties = schemas[0]["dataSchema"]["properties"]
        .as_array()
        .expect("inspect schema properties should be an array");
    assert!(properties.iter().any(|item| item["name"] == "agent"));
    assert!(properties.iter().any(|item| item["name"] == "capabilities"));
    assert!(properties.iter().any(|item| item["name"] == "inspection"));
    let required = schemas[0]["dataSchema"]["required"]
        .as_array()
        .expect("inspect schema required should be an array");
    assert!(required.iter().any(|item| item == "agent"));
    assert!(required.iter().any(|item| item == "capabilities"));
    assert!(required.iter().any(|item| item == "inspection"));
}

#[test]
fn schema_unknown_target_returns_invalid_argument() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "schema", "missing-command"]);

    assert_eq!(output.status.code(), Some(2));
    let json = stdout_json(&output);
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
    assert!(
        json["error"]["message"]
            .as_str()
            .expect("message should exist")
            .contains("Unknown schema target")
    );
    assert_eq!(json["error"]["details"]["command"], "missing-command");
}

#[test]
fn schema_ndjson_emits_single_result_event() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--output", "ndjson", "schema", "doctor"]);

    assert!(output.status.success());
    let lines = stdout_json_lines(&output);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["type"], "result");
    assert_eq!(lines[0]["action"], "schema");
    assert_eq!(lines[0]["meta"]["mode"], "ndjson");
    assert_eq!(lines[0]["data"]["data"]["commands"][0]["name"], "doctor");
}

#[test]
fn schema_upgrade_reflects_channel_and_version_fields() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "schema", "upgrade"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let schema = &json["data"]["commands"][0]["dataSchema"]["properties"];
    assert!(schema.as_array().is_some());
    let properties = schema.as_array().expect("properties should be an array");
    assert!(properties.iter().any(|item| item["name"] == "channel"));
    assert!(
        properties
            .iter()
            .any(|item| item["name"] == "currentVersion")
    );
    assert!(
        properties
            .iter()
            .any(|item| item["name"] == "latestVersion")
    );
    assert!(properties.iter().any(|item| item["name"] == "recoveryHint"));
    assert!(!properties.iter().any(|item| item["name"] == "command"));
    assert!(!properties.iter().any(|item| item["name"] == "dryRun"));
    assert!(!properties.iter().any(|item| item["name"] == "message"));
    assert!(!properties.iter().any(|item| item["name"] == "packageName"));
    assert!(
        !properties
            .iter()
            .any(|item| item["name"] == "verifiedVersion")
    );
}

#[test]
fn schema_doctor_describes_machine_actionable_issue_fields() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "schema", "doctor"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let required = json["data"]["commands"][0]["dataSchema"]["required"]
        .as_array()
        .expect("doctor required should be an array");
    let properties = json["data"]["commands"][0]["dataSchema"]["properties"]
        .as_array()
        .expect("doctor properties should be an array");
    assert!(required.iter().any(|item| item == "agents"));
    assert!(required.iter().any(|item| item == "installers"));
    assert!(required.iter().any(|item| item == "issues"));
    assert!(required.iter().any(|item| item == "self"));
    assert!(!properties.iter().any(|item| item["name"] == "checks"));
    assert!(
        !properties
            .iter()
            .any(|item| item["name"] == "installSource")
    );
    assert!(!properties.iter().any(|item| item["name"] == "ok"));
    assert!(!properties.iter().any(|item| item["name"] == "paths"));
    assert!(!properties.iter().any(|item| item["name"] == "summary"));
    let issues = properties
        .iter()
        .find(|item| item["name"] == "issues")
        .expect("issues property should exist");
    let issue_properties = issues["schema"]["items"]["properties"]
        .as_array()
        .expect("issue properties should be an array");

    assert!(
        issue_properties
            .iter()
            .any(|item| item["name"] == "suggestedAction")
    );
    assert!(
        issue_properties
            .iter()
            .any(|item| item["name"] == "suggestedCommands")
    );
    assert!(
        issue_properties
            .iter()
            .any(|item| item["name"] == "docsRef")
    );
}

#[test]
fn schema_exec_and_resolve_include_install_guidance_fields() {
    let workspace = TestWorkspace::new();

    let exec_output = run_agx(&workspace, &["--json", "schema", "exec"]);
    assert!(exec_output.status.success());
    let exec_json = stdout_json(&exec_output);
    let exec_properties = exec_json["data"]["commands"][0]["dataSchema"]["properties"]
        .as_array()
        .expect("exec properties should be an array");
    let execution = exec_properties
        .iter()
        .find(|item| item["name"] == "execution")
        .expect("exec execution should exist");
    let execution_properties = execution["schema"]["properties"]
        .as_array()
        .expect("execution properties should be an array");
    let install_guidance = execution_properties
        .iter()
        .find(|item| item["name"] == "installGuidance")
        .expect("exec installGuidance should exist");
    let exec_guidance_properties = install_guidance["schema"]["properties"]
        .as_array()
        .expect("exec install guidance properties should be an array");
    assert!(
        exec_guidance_properties
            .iter()
            .any(|item| item["name"] == "suggestedExecCommand")
    );
    assert!(
        execution_properties
            .iter()
            .any(|item| item["name"] == "installPolicy")
    );
    assert!(
        execution_properties
            .iter()
            .any(|item| item["name"] == "installed")
    );
    assert!(
        execution_properties
            .iter()
            .any(|item| item["name"] == "interactive")
    );
    assert!(
        execution_properties
            .iter()
            .any(|item| item["name"] == "launched")
    );
    assert!(
        exec_guidance_properties
            .iter()
            .any(|item| item["name"] == "installMethods")
    );
    assert!(
        !execution_properties
            .iter()
            .any(|item| item["name"] == "stdout")
    );
    assert!(
        !execution_properties
            .iter()
            .any(|item| item["name"] == "stderr")
    );

    let resolve_output = run_agx(&workspace, &["--json", "schema", "resolve"]);
    assert!(resolve_output.status.success());
    let resolve_json = stdout_json(&resolve_output);
    let resolve_properties = resolve_json["data"]["commands"][0]["dataSchema"]["properties"]
        .as_array()
        .expect("resolve properties should be an array");
    let resolution = resolve_properties
        .iter()
        .find(|item| item["name"] == "resolution")
        .expect("resolution property should exist");
    let resolution_properties = resolution["schema"]["properties"]
        .as_array()
        .expect("resolution properties should be an array");
    let install_guidance = resolution_properties
        .iter()
        .find(|item| item["name"] == "installGuidance")
        .expect("resolve installGuidance should exist");
    let resolve_guidance_properties = install_guidance["schema"]["properties"]
        .as_array()
        .expect("resolve install guidance properties should be an array");
    assert!(
        resolution_properties
            .iter()
            .any(|item| item["name"] == "installed")
    );
    assert!(
        resolve_guidance_properties
            .iter()
            .any(|item| item["name"] == "suggestedEnsureCommand")
    );
}

#[test]
fn schema_capabilities_and_commands_describe_nested_contracts() {
    let workspace = TestWorkspace::new();

    let capabilities_output = run_agx(&workspace, &["--json", "schema", "capabilities"]);
    assert!(capabilities_output.status.success());
    let capabilities_json = stdout_json(&capabilities_output);
    let capabilities_properties =
        capabilities_json["data"]["commands"][0]["dataSchema"]["properties"]
            .as_array()
            .expect("capabilities properties should be an array");
    let features = capabilities_properties
        .iter()
        .find(|item| item["name"] == "features")
        .expect("features property should exist");
    let feature_properties = features["schema"]["properties"]
        .as_array()
        .expect("feature properties should be an array");
    assert!(
        feature_properties
            .iter()
            .any(|item| item["name"] == "execInstallPolicies")
    );
    assert!(
        feature_properties
            .iter()
            .any(|item| item["name"] == "colorModes")
    );

    let commands_output = run_agx(&workspace, &["--json", "schema", "commands"]);
    assert!(commands_output.status.success());
    let commands_json = stdout_json(&commands_output);
    let commands_properties = commands_json["data"]["commands"][0]["dataSchema"]["properties"]
        .as_array()
        .expect("commands properties should be an array");
    let commands = commands_properties
        .iter()
        .find(|item| item["name"] == "commands")
        .expect("commands property should exist");
    let descriptor_properties = commands["schema"]["items"]["properties"]
        .as_array()
        .expect("descriptor properties should be an array");
    assert!(
        descriptor_properties
            .iter()
            .any(|item| item["name"] == "outputSchemaRef")
    );
    assert!(
        descriptor_properties
            .iter()
            .any(|item| item["name"] == "stability")
    );
}

#[test]
fn schema_envelope_and_ndjson_meta_require_core_fields() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "schema", "commands"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let command = &json["data"]["commands"][0];
    let envelope_required = command["envelopeSchema"]["required"]
        .as_array()
        .expect("envelope required should be an array");
    assert!(envelope_required.iter().any(|item| item == "action"));
    assert!(envelope_required.iter().any(|item| item == "error"));
    assert!(envelope_required.iter().any(|item| item == "meta"));
    assert!(envelope_required.iter().any(|item| item == "ok"));
    assert!(envelope_required.iter().any(|item| item == "warnings"));

    let meta = command["envelopeSchema"]["properties"]
        .as_array()
        .expect("envelope properties should be an array")
        .iter()
        .find(|item| item["name"] == "meta")
        .expect("meta property should exist");
    let meta_required = meta["schema"]["required"]
        .as_array()
        .expect("meta required should be an array");
    assert!(meta_required.iter().any(|item| item == "mode"));
    assert!(meta_required.iter().any(|item| item == "runId"));
    assert!(meta_required.iter().any(|item| item == "schemaVersion"));
    assert!(meta_required.iter().any(|item| item == "timestamp"));
    assert!(meta_required.iter().any(|item| item == "version"));

    let error = command["envelopeSchema"]["properties"]
        .as_array()
        .expect("envelope properties should be an array")
        .iter()
        .find(|item| item["name"] == "error")
        .expect("error property should exist");
    let error_properties = error["schema"]["properties"]
        .as_array()
        .expect("error properties should be an array");
    assert!(error_properties.iter().any(|item| item["name"] == "code"));
    assert!(
        error_properties
            .iter()
            .any(|item| item["name"] == "details")
    );
    assert!(
        error_properties
            .iter()
            .any(|item| item["name"] == "message")
    );

    let ndjson_required = command["ndjsonEventSchema"]["required"]
        .as_array()
        .expect("ndjson required should be an array");
    assert!(ndjson_required.iter().any(|item| item == "action"));
    assert!(ndjson_required.iter().any(|item| item == "meta"));
    assert!(ndjson_required.iter().any(|item| item == "type"));
}

#[test]
fn schema_optional_fields_are_not_misclassified_as_required() {
    let workspace = TestWorkspace::new();

    let commands_output = run_agx(&workspace, &["--json", "schema", "commands"]);
    assert!(commands_output.status.success());
    let commands_json = stdout_json(&commands_output);
    let commands_schema = &commands_json["data"]["commands"][0];

    let envelope_properties = commands_schema["envelopeSchema"]["properties"]
        .as_array()
        .expect("envelope properties should be an array");
    let error_schema = envelope_properties
        .iter()
        .find(|item| item["name"] == "error")
        .expect("error schema should exist");
    let error_required = error_schema["schema"]["required"]
        .as_array()
        .expect("error required should be an array");
    assert!(error_required.iter().any(|item| item == "code"));
    assert!(error_required.iter().any(|item| item == "message"));
    assert!(!error_required.iter().any(|item| item == "details"));

    let target_schema = envelope_properties
        .iter()
        .find(|item| item["name"] == "target")
        .expect("target schema should exist");
    let target_required = target_schema["schema"]["required"]
        .as_array()
        .expect("target required should be an array");
    assert!(target_required.iter().any(|item| item == "kind"));
    assert!(!target_required.iter().any(|item| item == "name"));

    let meta_schema = envelope_properties
        .iter()
        .find(|item| item["name"] == "meta")
        .expect("meta schema should exist");
    let meta_required = meta_schema["schema"]["required"]
        .as_array()
        .expect("meta required should be an array");
    assert!(!meta_required.iter().any(|item| item == "source"));
    assert!(!meta_required.iter().any(|item| item == "fetchedAt"));
    assert!(!meta_required.iter().any(|item| item == "staleAfter"));

    let exec_output = run_agx(&workspace, &["--json", "schema", "exec"]);
    assert!(exec_output.status.success());
    let exec_json = stdout_json(&exec_output);
    let exec_schema = &exec_json["data"]["commands"][0]["dataSchema"]["properties"];
    let exec_agent = exec_schema
        .as_array()
        .expect("exec properties should be an array")
        .iter()
        .find(|item| item["name"] == "agent")
        .expect("exec agent schema should exist");
    let exec_agent_required = exec_agent["schema"]["required"]
        .as_array()
        .expect("exec agent required should be an array");
    assert!(exec_agent_required.iter().any(|item| item == "name"));
    assert!(!exec_agent_required.iter().any(|item| item == "binaryName"));
    assert!(!exec_agent_required.iter().any(|item| item == "displayName"));

    let exec_execution = exec_schema
        .as_array()
        .expect("exec properties should be an array")
        .iter()
        .find(|item| item["name"] == "execution")
        .expect("exec execution schema should exist");
    let exec_required = exec_execution["schema"]["required"]
        .as_array()
        .expect("exec execution required should be an array");
    assert!(exec_required.iter().any(|item| item == "args"));
    assert!(exec_required.iter().any(|item| item == "installPolicy"));
    assert!(exec_required.iter().any(|item| item == "installed"));
    assert!(exec_required.iter().any(|item| item == "interactive"));
    assert!(exec_required.iter().any(|item| item == "launched"));
    assert!(!exec_required.iter().any(|item| item == "installGuidance"));

    let resolve_output = run_agx(&workspace, &["--json", "schema", "resolve"]);
    assert!(resolve_output.status.success());
    let resolve_json = stdout_json(&resolve_output);
    let resolve_schema = &resolve_json["data"]["commands"][0]["dataSchema"]["properties"];
    let resolution = resolve_schema
        .as_array()
        .expect("resolve properties should be an array")
        .iter()
        .find(|item| item["name"] == "resolution")
        .expect("resolve resolution schema should exist");
    let resolution_required = resolution["schema"]["required"]
        .as_array()
        .expect("resolution required should be an array");
    assert!(resolution_required.iter().any(|item| item == "installed"));
    assert!(
        resolution_required
            .iter()
            .any(|item| item == "installSource")
    );
    assert!(resolution_required.iter().any(|item| item == "lifecycle"));
    assert!(resolution_required.iter().any(|item| item == "sourceLabel"));
    assert!(
        resolution_required
            .iter()
            .any(|item| item == "suggestedLaunchCommand")
    );
    assert!(!resolution_required.iter().any(|item| item == "binaryPath"));
    assert!(
        !resolution_required
            .iter()
            .any(|item| item == "installGuidance")
    );
    assert!(
        !resolution_required
            .iter()
            .any(|item| item == "installedVersion")
    );

    let info_output = run_agx(&workspace, &["--json", "schema", "info"]);
    assert!(info_output.status.success());
    let info_json = stdout_json(&info_output);
    let info_schema = &info_json["data"]["commands"][0]["dataSchema"]["properties"];
    let inspection = info_schema
        .as_array()
        .expect("info properties should be an array")
        .iter()
        .find(|item| item["name"] == "inspection")
        .expect("inspection schema should exist");
    let inspection_required = inspection["schema"]["required"]
        .as_array()
        .expect("inspection required should be an array");
    assert!(inspection_required.iter().any(|item| item == "installed"));
    assert!(inspection_required.iter().any(|item| item == "lifecycle"));
    assert!(!inspection_required.iter().any(|item| item == "sourceLabel"));
    assert!(!inspection_required.iter().any(|item| item == "updateLabel"));
    assert!(!inspection_required.iter().any(|item| item == "binaryPath"));
    assert!(
        !inspection_required
            .iter()
            .any(|item| item == "installedVersion")
    );
    assert!(
        !inspection_required
            .iter()
            .any(|item| item == "latestVersion")
    );
    let inspection_properties = inspection["schema"]["properties"]
        .as_array()
        .expect("inspection properties should be an array");
    assert!(
        inspection_properties
            .iter()
            .any(|item| item["name"] == "sourceLabel")
    );
    assert!(
        !inspection_properties
            .iter()
            .any(|item| item["name"] == "updateLabel")
    );
}

#[test]
fn command_meta_timestamp_uses_iso_8601_shape() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "commands"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let timestamp = json["meta"]["timestamp"]
        .as_str()
        .expect("timestamp should exist");
    assert!(timestamp.contains('T'));
    assert!(timestamp.ends_with('Z'));
}

#[test]
fn schema_info_and_update_describe_payload_fields() {
    let workspace = TestWorkspace::new();

    let info_output = run_agx(&workspace, &["--json", "schema", "info"]);
    assert!(info_output.status.success());
    let info_json = stdout_json(&info_output);
    let info_properties = info_json["data"]["commands"][0]["dataSchema"]["properties"]
        .as_array()
        .expect("info properties should be an array");
    let agent = info_properties
        .iter()
        .find(|item| item["name"] == "agent")
        .expect("agent property should exist");
    let agent_properties = agent["schema"]["properties"]
        .as_array()
        .expect("agent properties should be an array");
    assert!(
        agent_properties
            .iter()
            .any(|item| item["name"] == "aliases")
    );
    assert!(
        agent_properties
            .iter()
            .any(|item| item["name"] == "selfUpdateCommands")
    );

    let update_output = run_agx(&workspace, &["--json", "schema", "update"]);
    assert!(update_output.status.success());
    let update_json = stdout_json(&update_output);
    let update_properties = update_json["data"]["commands"][0]["dataSchema"]["properties"]
        .as_array()
        .expect("update properties should be an array");
    let results = update_properties
        .iter()
        .find(|item| item["name"] == "results")
        .expect("results property should exist");
    let result_properties = results["schema"]["items"]["properties"]
        .as_array()
        .expect("update result properties should be an array");
    assert!(
        result_properties
            .iter()
            .any(|item| item["name"] == "strategy")
    );
    assert!(
        result_properties
            .iter()
            .any(|item| item["name"] == "installedVersion")
    );
    assert!(
        result_properties
            .iter()
            .any(|item| item["name"] == "latestVersion")
    );
}

#[test]
fn schema_install_describes_single_and_batch_payload_fields() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["--json", "schema", "install"]);

    assert!(output.status.success());
    let json = stdout_json(&output);
    let properties = json["data"]["commands"][0]["dataSchema"]["properties"]
        .as_array()
        .expect("install properties should be an array");

    assert!(properties.iter().any(|item| item["name"] == "agent"));
    assert!(properties.iter().any(|item| item["name"] == "message"));
    assert!(properties.iter().any(|item| item["name"] == "results"));
    assert!(properties.iter().any(|item| item["name"] == "scope"));
    assert!(properties.iter().any(|item| item["name"] == "summary"));

    let results = properties
        .iter()
        .find(|item| item["name"] == "results")
        .expect("results property should exist");
    let result_properties = results["schema"]["items"]["properties"]
        .as_array()
        .expect("install result properties should be an array");
    assert!(result_properties.iter().any(|item| item["name"] == "input"));
    assert!(result_properties.iter().any(|item| item["name"] == "ok"));
    assert!(
        result_properties
            .iter()
            .any(|item| item["name"] == "warnings")
    );

    let summary = properties
        .iter()
        .find(|item| item["name"] == "summary")
        .expect("summary property should exist");
    let summary_properties = summary["schema"]["properties"]
        .as_array()
        .expect("install summary properties should be an array");
    assert!(
        summary_properties
            .iter()
            .any(|item| item["name"] == "installed")
    );
    assert!(
        summary_properties
            .iter()
            .any(|item| item["name"] == "trackedExistingInstall")
    );
}

#[test]
fn human_commands_output_stays_readable() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["commands"]);

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("AGX Commands"));
    assert!(!stdout.contains("agx commands --json"));
}
