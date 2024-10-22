use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct TextCredentialParameter {
    pub value: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SshCredentialParameter {
    pub username: String,
    pub private_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CredentialType {
    Text(TextCredentialParameter),
    Ssh(SshCredentialParameter),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Credential {
    pub id: String,
    pub value: CredentialType,
    pub read_only: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for Credential {
    fn default() -> Self {
        Credential {
            id: String::new(),
            value: CredentialType::Text(TextCredentialParameter {
                value: String::new(),
            }),
            read_only: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Credential {
    pub fn get(credential_id: &str) -> Option<Self> {
        let path = default_credentials_location().join(format!("{}.yml", credential_id));
        if path.exists() {
            let content = std::fs::read_to_string(&path).expect("Could not read file");
            serde_yaml::from_str(&content).expect("Could not parse file")
        } else {
            None
        }
    }

    /// Reads as YamlCredential and converts to Credential. Primarily used for creating a new credential.
    pub fn read_from_yml(path_str: &str) -> Option<Self> {
        let path = PathBuf::from(path_str);
        if path.exists() {
            let yaml_credential: YamlCredential =
                serde_yaml::from_str(&std::fs::read_to_string(&path).expect("Could not read file"))
                    .expect("Could not parse file");
            Credential::try_from(yaml_credential).ok()
        } else {
            None
        }
    }

    pub fn save(&self) {
        let path = default_credentials_location().join(format!("{}.yml", self.id));
        let file = std::fs::File::create(&path).expect("Could not create file");
        let writer = std::io::BufWriter::new(file);
        serde_yaml::to_writer(writer, self).expect("Could not write to file");
    }
}

#[derive(Deserialize, Debug)]
pub struct YamlCredential {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub value: serde_yaml::Value,
    pub read_only: bool,
}

impl TryFrom<YamlCredential> for Credential {
    type Error = String;

    fn try_from(value: YamlCredential) -> Result<Self, Self::Error> {
        let c_type = match value.type_.as_str() {
            "text" => {
                let text_value: TextCredentialParameter =
                    serde_yaml::from_value(value.value).map_err(|e| e.to_string())?;
                CredentialType::Text(text_value)
            }
            "ssh" => {
                let ssh_value: SshCredentialParameter =
                    serde_yaml::from_value(value.value).map_err(|e| e.to_string())?;
                CredentialType::Ssh(ssh_value)
            }
            _ => return Err("Invalid credential type".to_string()),
        };

        Ok(Credential {
            id: value.id,
            value: c_type,
            read_only: value.read_only,
            ..Default::default()
        })
    }
}

impl TryFrom<&YamlCredential> for Credential {
    type Error = String;

    fn try_from(value: &YamlCredential) -> Result<Self, Self::Error> {
        let c_type = match value.type_.as_str() {
            "text" => {
                let text_value: TextCredentialParameter =
                    serde_yaml::from_value(value.value.clone()).map_err(|e| e.to_string())?;
                CredentialType::Text(text_value)
            }
            "ssh" => {
                let ssh_value: SshCredentialParameter =
                    serde_yaml::from_value(value.value.clone()).map_err(|e| e.to_string())?;
                CredentialType::Ssh(ssh_value)
            }
            _ => return Err("Invalid credential type".to_string()),
        };

        Ok(Credential {
            id: value.id.clone(),
            value: c_type,
            read_only: value.read_only,
            ..Default::default()
        })
    }
}

impl TryFrom<PathBuf> for YamlCredential {
    type Error = String;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let file = std::fs::File::open(&path).map_err(|_| "Could not open file")?;
        let reader = std::io::BufReader::new(file);
        let yaml: serde_yaml::Value = serde_yaml::from_reader(reader).map_err(|e| e.to_string())?;

        let yaml_credential: YamlCredential =
            serde_yaml::from_value(yaml).map_err(|e| e.to_string())?;

        Ok(yaml_credential)
    }
}

pub fn default_credentials_location() -> PathBuf {
    let path = if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").expect("Could not get APPDATA");
        PathBuf::from(appdata).join("nomos").join("credentials")
    } else {
        PathBuf::from("/var/lib/nomos/credentials")
    };
    std::fs::create_dir_all(&path).expect("Could not create credentials directory");
    path
}
