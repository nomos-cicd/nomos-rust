use std::path::PathBuf;

use serde::Deserialize;

use crate::{
    credential::{Credential, YamlCredential},
    job::{Job, JobResult},
    log::LogLevel,
    script::models::Script,
};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub credentials: Vec<YamlCredential>,
}

impl Settings {
    pub fn sync(&self, job_result: &mut JobResult) -> Result<(), String> {
        let mut credential_ids: Vec<String> = Vec::new();
        for yaml_credential in &self.credentials {
            let credential = Credential::try_from(yaml_credential);
            if let Err(e) = credential {
                job_result.add_log(LogLevel::Error, format!("Error syncing credential: {:?}", e));
                continue;
            }
            let credential = credential.unwrap();
            if credential.read_only {
                job_result.add_log(
                    LogLevel::Info,
                    format!("Skipping read-only credential {:?}", credential.id),
                );
                continue;
            }

            let res = credential.sync(job_result.into());
            if let Err(e) = res {
                job_result.add_log(LogLevel::Error, format!("Error syncing credential: {:?}", e));
                continue;
            }
            credential_ids.push(yaml_credential.id.clone());
        }

        let credentials = Credential::get_all()?;
        for credential in credentials {
            if !credential_ids.contains(&credential.id) && !credential.read_only {
                let res = credential.delete();
                if let Err(e) = res {
                    job_result.add_log(LogLevel::Error, format!("Error deleting credential: {:?}", e));
                    continue;
                }
            }
        }

        Ok(())
    }
}

impl TryFrom<PathBuf> for Settings {
    type Error = String;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let yaml_str = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let settings: Settings = serde_yaml::from_str(yaml_str.as_str()).map_err(|e| e.to_string())?;
        Ok(settings)
    }
}

pub fn sync(directory: PathBuf, job_result: &mut JobResult) -> Result<(), String> {
    if job_result.dry_run {
        job_result.add_log(LogLevel::Info, "Dry run enabled, skipping sync".to_string());
        return Ok(());
    }

    let settings_path = directory.join("settings.yml");
    if settings_path.exists() {
        let settings = Settings::try_from(settings_path)?;
        settings.sync(job_result)?;
    } else {
        job_result.add_log(LogLevel::Info, "No settings file found".to_string());
    }

    let scripts_path = directory.join("scripts");
    if scripts_path.exists() {
        let mut script_ids: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(scripts_path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let script = Script::try_from(path);
            if let Err(e) = script {
                job_result.add_log(LogLevel::Error, format!("Error syncing script: {:?}", e));
                continue;
            }
            let script = script.unwrap();
            let res = script.sync(job_result.into());
            if let Err(e) = res {
                job_result.add_log(LogLevel::Error, format!("Error syncing script: {:?}", e));
                continue;
            }
            script_ids.push(script.id.clone());
        }
        let scripts = Script::get_all()?;
        for script in scripts {
            if !script_ids.contains(&script.id) {
                let res = script.delete();
                if let Err(e) = res {
                    job_result.add_log(LogLevel::Error, format!("Error deleting script: {:?}", e));
                    continue;
                }
                job_result.add_log(LogLevel::Info, format!("Deleted script {:?}", script.id));
            }
        }
    } else {
        job_result.add_log(LogLevel::Info, "No scripts directory found".to_string());
    }

    let jobs_path = directory.join("jobs");
    if jobs_path.exists() {
        let mut job_ids: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(jobs_path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let job = Job::try_from(path);
            if let Err(e) = job {
                job_result.add_log(LogLevel::Error, format!("Error syncing job: {:?}", e));
                continue;
            }
            let job = job.unwrap();
            if job.read_only {
                job_result.add_log(LogLevel::Info, format!("Skipping read-only job {:?}", job.id));
                continue;
            }
            let res = job.sync(job_result.into());
            if let Err(e) = res {
                job_result.add_log(LogLevel::Error, format!("Error syncing job: {:?}", e));
                continue;
            }
            job_ids.push(job.id.clone());
        }
        let jobs = Job::get_all()?;
        for job in jobs {
            if !job_ids.contains(&job.id) && !job.read_only {
                let res = job.delete();
                if let Err(e) = res {
                    job_result.add_log(LogLevel::Error, format!("Error deleting job: {:?}", e));
                    continue;
                }
                job_result.add_log(LogLevel::Info, format!("Deleted job {:?}", job.id));
            }
        }
    } else {
        job_result.add_log(LogLevel::Info, "No jobs directory found".to_string());
    }

    Ok(())
}
