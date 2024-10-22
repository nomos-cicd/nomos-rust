use std::path::PathBuf;

use chrono::Utc;
use nomos_rust::credential::{Credential, CredentialType, TextCredentialParameter, YamlCredential};

#[test]
fn create_credential() {
    let text_credential = TextCredentialParameter {
        value: "test".to_string(),
    };

    let yaml_credential = YamlCredential {
        id: "test-credential".to_string(),
        value: serde_yaml::to_value(text_credential).unwrap(),
        read_only: false,
        type_: "text".to_string(),
    };

    let credential = Credential::try_from(yaml_credential);
    assert!(credential.is_ok());
    let credential = credential.unwrap();
    assert_eq!(credential.id, "test-credential");
    match &credential.value {
        CredentialType::Text(_) => {}
        _ => panic!("Expected text credential"),
    }
    assert!(!credential.read_only);
    assert!(credential.created_at < Utc::now());
    assert!(credential.updated_at < Utc::now());
}

#[test]
fn read_yml() {
    let path_buf = PathBuf::from("tests/credentials/test-credential.yml");
    let yaml_credential = YamlCredential::try_from(path_buf);
    assert!(yaml_credential.is_ok());

    let credential = Credential::try_from(yaml_credential.unwrap());
    assert!(credential.is_ok());
    let credential = credential.unwrap();
    assert_eq!(credential.id, "test-credential");
    match &credential.value {
        CredentialType::Text(_) => {}
        _ => panic!("Expected text credential"),
    }
    assert!(!credential.read_only);
    assert!(credential.created_at < Utc::now());
    assert!(credential.updated_at < Utc::now());
}
