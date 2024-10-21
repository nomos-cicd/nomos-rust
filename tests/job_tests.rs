use std::path::PathBuf;

use nomos_rust::job::{Job, TriggerType, YamlJob, YamlTrigger};

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
    let path_buf = PathBuf::from("tests/test-job.yml");
    let yaml_job = YamlJob::try_from(path_buf);
    assert!(yaml_job.is_ok());

    let job = Job::try_from(yaml_job.unwrap());
    assert!(job.is_ok());
}
