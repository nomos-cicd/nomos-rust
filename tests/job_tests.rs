use std::path::PathBuf;

use nomos_rust::job::{Job, TriggerType, YamlJob, YamlTrigger};
use nomos_rust::script::Script;

#[test]
fn create_job() {
    let manual_trigger = YamlTrigger {
        type_: "manual".to_string(),
        value: serde_yaml::Value::Null,
    };

    let yaml_job = YamlJob {
        id: "test".to_string(),
        name: "Test".to_string(),
        parameters: vec![],
        triggers: vec![manual_trigger],
        script_id: "test".to_string(),
    };

    let job = Job::try_from(yaml_job);
    assert!(job.is_ok());
    let job = job.unwrap();
    assert_eq!(job.id, "test");
    assert_eq!(job.name, "Test");
    assert_eq!(job.parameters.len(), 0);
    assert_eq!(job.triggers.len(), 1);
    match &job.triggers[0].value {
        TriggerType::Manual(_) => {}
        _ => panic!("Expected manual trigger"),
    }
}

#[test]
fn read_yml() {
    let path_buf = PathBuf::from("tests/jobs/test-job.yml");
    let yaml_job = YamlJob::try_from(path_buf);
    assert!(yaml_job.is_ok());

    let job = Job::try_from(yaml_job.unwrap());
    assert!(job.is_ok());
}

#[test]
fn execute_job() {
    let path_buf = PathBuf::from("tests/jobs/test-job.yml");
    let yaml_job = YamlJob::try_from(path_buf).unwrap();
    let job = Job::try_from(yaml_job).unwrap();
    let script = Script::get_from_path("tests/scripts/test-script.yml").unwrap();
    let result = job.execute_with_script(Default::default(), &script);
    assert!(result.finished_at.is_some());
    assert!(result.is_success);
    assert_eq!(result.steps.len(), 1);
    assert_eq!(result.current_step.unwrap().name, "Test Step");
    assert!(result.finished_at.unwrap() > result.started_at);
}

#[test]
fn git_clone_job() {
    let path_buf = PathBuf::from("tests/jobs/git-clone-job.yml");
    let yaml_job = YamlJob::try_from(path_buf).unwrap();
    let job = Job::try_from(yaml_job).unwrap();
    let script = Script::get_from_path("tests/scripts/git-clone-script.yml").unwrap();
    let result = job.execute_with_script(Default::default(), &script);
    assert!(result.finished_at.is_some());
    assert!(result.is_success);
    assert_eq!(result.steps.len(), 1);
}
