use std::path::PathBuf;

use nomos_rust::credential::{Credential, CredentialType, TextCredentialParameter};

#[test]
fn create_credential() {
    let text_credential = TextCredentialParameter {
        value: "test".to_string(),
    };

    let yaml_credential = Credential {
        id: "test-credential".to_string(),
        read_only: false,
        value: CredentialType::Text(text_credential),
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
}

#[test]
fn read_yml() {
    let path_buf = PathBuf::from("tests/credentials/test-credential.yml");
    let credential = Credential::try_from(path_buf);
    assert!(credential.is_ok());
    let credential = credential.unwrap();
    assert_eq!(credential.id, "test-credential");
    match &credential.value {
        CredentialType::Text(_) => {}
        _ => panic!("Expected text credential"),
    }
    assert!(!credential.read_only);
}
