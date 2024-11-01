use std::path::PathBuf;

use chrono::Utc;
use nomos_rust::{
    job::{Job, JobResult},
    script::models::Script,
    settings,
};

#[tokio::test]
async fn sync() {
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
    job_result.save().unwrap(); // Workaround for creating yml file.
    let res = settings::sync(path, &mut job_result).await;
    job_result.save().unwrap(); // Workaround for creating yml file.
    assert!(res.is_ok());
    job_result.finished_at = Some(Utc::now());
    job_result.is_success = res.is_ok();
    job_result.save().unwrap(); // Workaround for creating yml file.
}
