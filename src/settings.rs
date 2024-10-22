use std::path::PathBuf;

use serde::Deserialize;

use crate::credential::{Credential, YamlCredential};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub yaml_credentials: Vec<YamlCredential>,
}

impl Settings {
    pub fn apply(&self) {
        for yaml_credential in &self.yaml_credentials {
            let credential = Credential::try_from(yaml_credential);
            if let Err(e) = credential {
                eprintln!("Error applying credential: {}", e);
                continue;
            }

            // Check if exists
            let existing_credential = Credential::get(credential.as_ref().unwrap().id.as_str());
            if let Some(existing_credential) = existing_credential {
                eprintln!("Credential {} already exists", existing_credential.id);
                continue;
            }

            credential.unwrap().save();
        }
    }
}

impl TryFrom<PathBuf> for Settings {
    type Error = String;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let yaml_str = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let settings: Settings =
            serde_yaml::from_str(yaml_str.as_str()).map_err(|e| e.to_string())?;
        Ok(settings)
    }
}
