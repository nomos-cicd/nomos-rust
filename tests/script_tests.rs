use std::path::PathBuf;

use nomos_rust::script::Script;

#[test]
fn read_yml() {
    let path_buf = PathBuf::from("tests/scripts/test-script.yml");
    let yaml_script = Script::try_from(path_buf);
    assert!(yaml_script.is_ok());
}
