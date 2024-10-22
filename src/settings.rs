use std::path::PathBuf;

use serde::Deserialize;

use crate::{
    credential::{Credential, YamlCredential},
    job::Job,
    script::Script,
};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub yaml_credentials: Vec<YamlCredential>,
}

impl Settings {
    pub fn sync(&self) {
        let mut credential_ids: Vec<String> = Vec::new();
        for yaml_credential in &self.yaml_credentials {
            let credential = Credential::try_from(yaml_credential);
            if let Err(e) = credential {
                eprintln!("Error applying credential: {}", e);
                continue;
            }

            credential.unwrap().sync();
            credential_ids.push(yaml_credential.id.clone());
        }

        let credentials = Credential::get_all();
        for credential in credentials {
            if !credential_ids.contains(&credential.id) && !credential.read_only {
                credential.delete();
            }
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

pub fn sync(directory: PathBuf) {
    let settings_path = directory.join("settings.yaml");
    let settings = Settings::try_from(settings_path).unwrap();
    settings.sync();

    let scripts_path = directory.join("scripts");
    for entry in std::fs::read_dir(scripts_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let script = Script::try_from(path).unwrap();
        script.sync();
    }

    let jobs_path = directory.join("jobs");
    for entry in std::fs::read_dir(jobs_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let job = Job::try_from(path).unwrap();
        job.sync();
    }
}
