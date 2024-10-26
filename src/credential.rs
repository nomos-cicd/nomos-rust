use std::{path::PathBuf, str::FromStr};

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{job::JobResult, log::LogLevel};

#[derive(Deserialize, Serialize, Clone, PartialEq, JsonSchema, Default, Debug)]
pub struct TextCredentialParameter {
    pub value: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, JsonSchema, Default, Debug)]
pub struct SshCredentialParameter {
    pub username: String,
    pub private_key: String,
}

/// Similar to node.js's `.env` file.
#[derive(Deserialize, Serialize, Clone, PartialEq, JsonSchema, Default, Debug)]
pub struct EnvCredentialParameter {
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(tag = "type")]
pub enum CredentialType {
    #[serde(rename = "text")]
    Text(TextCredentialParameter),
    #[serde(rename = "ssh")]
    Ssh(SshCredentialParameter),
    #[serde(rename = "env")]
    Env(EnvCredentialParameter),
}

impl CredentialType {
    pub fn get_json_schema() -> serde_json::Value {
        let schema = schemars::schema_for!(CredentialType);
        serde_json::to_value(schema).unwrap()
    }
}

impl FromStr for CredentialType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(CredentialType::Text(TextCredentialParameter::default())),
            "ssh" => Ok(CredentialType::Ssh(SshCredentialParameter::default())),
            "env" => Ok(CredentialType::Env(EnvCredentialParameter::default())),
            _ => Err(format!("Unknown credential type: {}", s)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Credential {
    pub id: String,
    pub value: CredentialType,
    pub read_only: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PartialEq for Credential {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.value == other.value && self.read_only == other.read_only
    }
}

impl Default for Credential {
    fn default() -> Self {
        Credential {
            id: String::new(),
            value: CredentialType::Text(TextCredentialParameter { value: String::new() }),
            read_only: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Credential {
    pub fn get(credential_id: &str, job_result: Option<&mut JobResult>) -> Result<Option<Self>, String> {
        let path = default_credentials_location()?.join(format!("{}.yml", credential_id));
        if path.exists() {
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let res: Result<Option<Self>, String> = serde_yaml::from_str(&content).map_err(|e| e.to_string()).map(Some);
            if let Some(job_result) = job_result {
                if let Ok(Some(mut res)) = res.clone() {
                    match res.value {
                        CredentialType::Text(ref mut text) => {
                            if text.value.is_empty() {
                                job_result
                                    .add_log(LogLevel::Warning, format!("Empty text credential: {}", credential_id));
                            }
                        }
                        CredentialType::Env(ref mut env) => {
                            if env.value.is_empty() {
                                job_result
                                    .add_log(LogLevel::Warning, format!("Empty env credential: {}", credential_id));
                            }
                        }
                        CredentialType::Ssh(ref mut ssh) => {
                            if ssh.username.is_empty() || ssh.private_key.is_empty() {
                                job_result
                                    .add_log(LogLevel::Warning, format!("Empty ssh credential: {}", credential_id));
                            }
                        }
                    }
                }
            }

            res
        } else {
            Ok(None)
        }
    }

    pub fn get_all() -> Result<Vec<Self>, String> {
        let path = default_credentials_location()?;
        let mut credentials = Vec::new();
        for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let credential = Credential::try_from(path).map_err(|e| e.to_string())?;
            credentials.push(credential);
        }
        Ok(credentials)
    }

    pub fn get_credential_type(&self) -> &str {
        match self.value {
            CredentialType::Text(_) => "text",
            CredentialType::Ssh(_) => "ssh",
            CredentialType::Env(_) => "env",
        }
    }

    // If job_result is null, it means we are doing from the API. Allow it.
    // If job_result is not null, it means we are doing from the job. Check if the credential is changed.
    pub fn sync(&self, job_result: &mut Option<&mut JobResult>) -> Result<(), String> {
        if job_result.is_none() {
            eprintln!("Syncing credential {:?}", self.id);
            self.save()?;
            return Ok(());
        }
        let job_result = job_result.as_deref_mut();
        let job_result = job_result.unwrap();

        let current_type = self.get_credential_type();
        let existing_credential = Credential::get(self.id.as_str(), Some(job_result))?;
        if let Some(existing_credential) = existing_credential {
            let existing_type = existing_credential.get_credential_type();
            if *existing_type != *current_type {
                self.save()?;
                job_result.add_log(LogLevel::Info, format!("Updated credential {:?}", self.id))
            } else {
                job_result.add_log(LogLevel::Info, format!("No changes in credential {:?}", self.id))
            }
        } else {
            self.save()?;
            job_result.add_log(LogLevel::Info, format!("Created credential {:?}", self.id))
        }

        Ok(())
    }

    fn save(&self) -> Result<(), String> {
        let path = default_credentials_location()?.join(format!("{}.yml", self.id));
        let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
        let writer = std::io::BufWriter::new(file);
        serde_yaml::to_writer(writer, self).map_err(|e| e.to_string())
    }

    pub fn delete(&self) -> Result<(), String> {
        let path = default_credentials_location()?.join(format!("{}.yml", self.id));
        std::fs::remove_file(path).map_err(|e| e.to_string())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct YamlCredential {
    pub id: String,
    pub read_only: bool,
    pub value: CredentialType,
}

impl TryFrom<YamlCredential> for Credential {
    type Error = String;

    fn try_from(value: YamlCredential) -> Result<Self, Self::Error> {
        Ok(Credential {
            id: value.id,
            value: value.value,
            read_only: value.read_only,
            ..Default::default()
        })
    }
}

impl TryFrom<&YamlCredential> for Credential {
    type Error = String;

    fn try_from(value: &YamlCredential) -> Result<Self, Self::Error> {
        Ok(Credential {
            id: value.id.clone(),
            value: value.value.clone(),
            read_only: value.read_only,
            ..Default::default()
        })
    }
}

impl TryFrom<PathBuf> for YamlCredential {
    type Error = String;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let file = std::fs::File::open(path).map_err(|_| "Could not open file")?;
        let reader = std::io::BufReader::new(file);
        let yaml: serde_yaml::Value = serde_yaml::from_reader(reader).map_err(|e| e.to_string())?;

        let yaml_credential: YamlCredential = serde_yaml::from_value(yaml).map_err(|e| e.to_string())?;

        Ok(yaml_credential)
    }
}

impl TryFrom<PathBuf> for Credential {
    type Error = String;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let yaml_credential = YamlCredential::try_from(path)?;
        Credential::try_from(yaml_credential)
    }
}

pub fn default_credentials_location() -> Result<PathBuf, String> {
    let path = if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").map_err(|e| e.to_string())?;
        PathBuf::from(appdata).join("nomos").join("credentials")
    } else {
        PathBuf::from("/var/lib/nomos/credentials")
    };
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path)
}
