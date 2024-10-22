use std::path::PathBuf;

use nomos_rust::job::Job;
use nomos_rust::script::Script;

#[test]
fn read_yml() {
    let path_buf = PathBuf::from("tests/jobs/test-job.yml");
    let job = Job::try_from(path_buf);
    assert!(job.is_ok());
}

#[test]
fn execute_job() {
    let path_buf = PathBuf::from("tests/jobs/test-job.yml");
    let job = Job::try_from(path_buf).unwrap();
    let script = Script::try_from(PathBuf::from("tests/scripts/test-script.yml")).unwrap();
    let result = job
        .execute_with_script(Default::default(), &script)
        .unwrap();
    assert!(result.finished_at.is_some());
    assert!(result.is_success);
    assert_eq!(result.steps.len(), 1);
    assert_eq!(result.current_step_name.unwrap(), "Test Step");
    assert!(result.finished_at.unwrap() > result.started_at);
}

#[test]
fn git_clone_job() {
    let path_buf = PathBuf::from("tests/jobs/git-clone-job.yml");
    let job = Job::try_from(path_buf).unwrap();
    let script = Script::try_from(PathBuf::from("tests/scripts/git-clone-script.yml")).unwrap();
    let result = job
        .execute_with_script(Default::default(), &script)
        .unwrap();
    assert!(result.finished_at.is_some());
    assert!(result.is_success);
    assert_eq!(result.steps.len(), 2);
}
