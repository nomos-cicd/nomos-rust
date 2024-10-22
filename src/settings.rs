use std::path::PathBuf;

use serde::Deserialize;

use crate::{
    credential::{Credential, YamlCredential},
    job::{Job, JobResult},
    log,
    script::Script,
};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub credentials: Vec<YamlCredential>,
}

impl Settings {
    pub fn sync(&self, job_result: &mut JobResult) {
        let mut credential_ids: Vec<String> = Vec::new();
        for yaml_credential in &self.credentials {
            let credential = Credential::try_from(yaml_credential);
            if let Err(e) = credential {
                job_result.add_log(
                    log::LogLevel::Error,
                    format!("Error syncing credential: {:?}", e),
                );
                continue;
            }

            credential.unwrap().sync(job_result);
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

pub fn sync(directory: PathBuf, job_result: &mut JobResult) -> Result<(), String> {
    let settings_path = directory.join("settings.yml");
    let settings = Settings::try_from(settings_path).unwrap();
    settings.sync(job_result);

    let mut script_ids: Vec<String> = Vec::new();
    let scripts_path = directory.join("scripts");
    for entry in std::fs::read_dir(scripts_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let script = Script::try_from(path).unwrap();
        script.sync(job_result);
        script_ids.push(script.id.clone());
    }
    let scripts = Script::get_all();
    for script in scripts {
        if !script_ids.contains(&script.id) {
            script.delete();
            job_result.add_log(
                log::LogLevel::Info,
                format!("Deleted script {:?}", script.id),
            );
        }
    }

    let mut job_ids: Vec<String> = Vec::new();
    let jobs_path = directory.join("jobs");
    for entry in std::fs::read_dir(jobs_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let job = Job::try_from(path).unwrap();
        job.sync(job_result);
        job_ids.push(job.id.clone());
    }
    let jobs = Job::get_all();
    for job in jobs {
        if !job_ids.contains(&job.id) && !job.read_only {
            job.delete();
            job_result.add_log(log::LogLevel::Info, format!("Deleted job {:?}", job.id));
        }
    }

    Ok(())
}
