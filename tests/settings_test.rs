use std::path::PathBuf;

use nomos_rust::{
    job::{Job, JobResult},
    script::models::Script,
    settings,
};

#[test]
fn sync() {
    let path = PathBuf::from("tests");
    let job: Job = Default::default();
    let script: Script = Default::default();
    let mut job_result = JobResult::try_from((&job, &script, false)).unwrap();
    let res = settings::sync(path, &mut job_result);
    assert!(res.is_ok());
}
