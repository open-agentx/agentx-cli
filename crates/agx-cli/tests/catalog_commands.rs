mod support;

use support::{TestWorkspace, run_agx, stdout_json, stdout_text};

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
fn human_commands_output_stays_readable() {
    let workspace = TestWorkspace::new();
    let output = run_agx(&workspace, &["commands"]);

    assert!(output.status.success());
    let stdout = stdout_text(&output);
    assert!(stdout.contains("AGX Commands"));
    assert!(stdout.contains("agx commands --json"));
}
