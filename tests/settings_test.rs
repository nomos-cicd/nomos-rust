use std::path::PathBuf;

use nomos_rust::settings;

#[test]
fn sync() {
    let path = PathBuf::from("tests");
    let res = settings::sync(path, &mut Default::default());
    assert!(res.is_ok());
}
