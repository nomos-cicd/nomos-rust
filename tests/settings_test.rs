use std::path::PathBuf;

use nomos_rust::{
    job::{Job, JobResult},
    script::models::Script,
    settings,
};

#[test]
fn sync() {
    let path = PathBuf::from("tests");
    let job = Job {
        id: "test-job".to_string(),
        name: "Test Job".to_string(),
        parameters: vec![],
        read_only: false,
        script_id: "test-script".to_string(),
        triggers: vec![],
    };
    let script = Script {
        id: "test-script".to_string(),
        name: "Test Script".to_string(),
        parameters: vec![],
        steps: vec![],
    };
    let mut job_result = JobResult::try_from((&job, &script, false)).unwrap();
    let res = settings::sync(path, &mut job_result);
    assert!(res.is_ok());
}
