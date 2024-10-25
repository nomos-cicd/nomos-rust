use nomos_rust::job::{Job, JobParameterDefinition};
use nomos_rust::script::models::Script;
use nomos_rust::script::{ScriptParameter, ScriptParameterType};

fn create_test_script(parameters: Vec<ScriptParameter>) -> Script {
    Script {
        id: "test-script".to_string(),
        name: "Test Script".to_string(),
        parameters,
        steps: vec![],
    }
}

#[test]
fn test_validate_parameters_empty() {
    let job = Job {
        id: "test-job".to_string(),
        name: "Test Job".to_string(),
        parameters: vec![],
        triggers: vec![],
        script_id: "test-script".to_string(),
        read_only: false,
    };

    let script = create_test_script(vec![]);
    assert!(job.validate_parameters(Some(&script)).is_ok());
}

#[test]
fn test_validate_parameters_required_provided() {
    let job = Job {
        id: "test-job".to_string(),
        name: "Test Job".to_string(),
        parameters: vec![JobParameterDefinition {
            name: "param1".to_string(),
            default: Some(ScriptParameterType::String("value1".to_string())),
        }],
        triggers: vec![],
        script_id: "test-script".to_string(),
        read_only: false,
    };

    let script = create_test_script(vec![ScriptParameter {
        name: "param1".to_string(),
        description: "Test parameter".to_string(),
        required: true,
        default: None,
    }]);

    assert!(job.validate_parameters(Some(&script)).is_ok());
}

#[test]
fn test_validate_parameters_required_missing() {
    let job = Job {
        id: "test-job".to_string(),
        name: "Test Job".to_string(),
        parameters: vec![],
        triggers: vec![],
        script_id: "test-script".to_string(),
        read_only: false,
    };

    let script = create_test_script(vec![ScriptParameter {
        name: "param1".to_string(),
        description: "Test parameter".to_string(),
        required: true,
        default: None,
    }]);

    assert!(job.validate_parameters(Some(&script)).is_err());
}

#[test]
fn test_validate_parameters_optional_missing() {
    let job = Job {
        id: "test-job".to_string(),
        name: "Test Job".to_string(),
        parameters: vec![],
        triggers: vec![],
        script_id: "test-script".to_string(),
        read_only: false,
    };

    let script = create_test_script(vec![ScriptParameter {
        name: "param1".to_string(),
        description: "Test parameter".to_string(),
        required: true,
        default: Some(ScriptParameterType::String("default".to_string())),
    }]);

    assert!(job.validate_parameters(Some(&script)).is_ok());
}

#[test]
fn test_validate_parameters_multiple() {
    let job = Job {
        id: "test-job".to_string(),
        name: "Test Job".to_string(),
        parameters: vec![JobParameterDefinition {
            name: "param1".to_string(),
            default: Some(ScriptParameterType::String("value1".to_string())),
        }],
        triggers: vec![],
        script_id: "test-script".to_string(),
        read_only: false,
    };

    let script = create_test_script(vec![
        ScriptParameter {
            name: "param1".to_string(),
            description: "Test parameter 1".to_string(),
            required: true,
            default: None,
        },
        ScriptParameter {
            name: "param2".to_string(),
            description: "Test parameter 2".to_string(),
            required: true,
            default: Some(ScriptParameterType::Number(42)),
        },
        ScriptParameter {
            name: "param3".to_string(),
            description: "Test parameter 3".to_string(),
            required: false,
            default: None,
        },
    ]);

    assert!(job.validate_parameters(Some(&script)).is_ok());
}

#[test]
fn test_validate_parameters_multiple_missing() {
    let job = Job {
        id: "test-job".to_string(),
        name: "Test Job".to_string(),
        parameters: vec![],
        triggers: vec![],
        script_id: "test-script".to_string(),
        read_only: false,
    };

    let script = create_test_script(vec![
        ScriptParameter {
            name: "param1".to_string(),
            description: "Test parameter 1".to_string(),
            required: true,
            default: None,
        },
        ScriptParameter {
            name: "param2".to_string(),
            description: "Test parameter 2".to_string(),
            required: true,
            default: None,
        },
    ]);

    let result = job.validate_parameters(Some(&script));
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Missing parameters: param1, param2");
}
