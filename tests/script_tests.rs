use std::path::PathBuf;

use nomos_rust::script::{Script, ScriptParameterType};

#[test]
fn read_yml() {
    let path_buf = PathBuf::from("tests/scripts/test-script.yml");
    let script = Script::try_from(path_buf);
    assert!(script.is_ok());
    let script = script.unwrap();
    assert_eq!(script.id, "test-script");
    assert_eq!(script.parameters.len(), 2);
    assert_eq!(script.parameters[0].name, "test_param1");
    assert_eq!(
        script.parameters[0].default,
        Some(ScriptParameterType::Number(5))
    );
    assert_eq!(script.parameters[1].name, "test_param2");
    assert_eq!(
        script.parameters[1].default,
        Some(ScriptParameterType::Boolean(true))
    );
}
