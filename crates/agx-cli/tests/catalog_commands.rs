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
fn commands_json_describes_install_and_inspect_flags() {
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

    assert!(install_flags.contains(&"--yes"));
    assert!(install_flags.contains(&"--dry-run"));
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
}

#[test]
fn human_commands_output_stays_readable() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["commands"]);

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("AGX Commands"));
    assert!(stdout.contains("agx commands --json"));
}
