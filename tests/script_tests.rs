use std::path::PathBuf;

use nomos_rust::script::{ScriptType, YamlScript, YamlScriptStep};

#[test]
fn create_script() {
    let yaml_script = YamlScript {
        id: "test".to_string(),
        name: "Test".to_string(),
        parameters: vec![],
        steps: vec![YamlScriptStep {
            name: "Step 1".to_string(),
            value: ScriptType::Bash(nomos_rust::script::BashScript {
                code: "echo 'Hello'".to_string(),
            }),
            ..Default::default()
        }],
    };

    let script = nomos_rust::script::Script::from(yaml_script);
    assert_eq!(script.id, "test");
    assert_eq!(script.name, "Test");
    assert_eq!(script.parameters.len(), 0);
    assert_eq!(script.steps.len(), 1);
    assert_eq!(script.steps[0].name, "Step 1");
    assert_eq!(script.steps[0].values.len(), 1);
    match &script.steps[0].values[0] {
        ScriptType::Bash(bash) => {
            assert_eq!(bash.code, "echo 'Hello'");
        }
        _ => panic!("Expected Bash script"),
    }
}

#[test]
fn read_yml() {
    let path_buf = PathBuf::from("tests/scripts/test-script.yml");
    let yaml_script = YamlScript::try_from(path_buf);
    assert!(yaml_script.is_ok());
}
