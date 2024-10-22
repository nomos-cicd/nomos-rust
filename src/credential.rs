use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
pub struct TextCredentialParameter {
    pub value: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
pub struct SshCredentialParameter {
    pub username: String,
    pub private_key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum CredentialType {
    #[serde(rename = "text")]
    Text(TextCredentialParameter),
    #[serde(rename = "ssh")]
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

impl PartialEq for Credential {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.value == other.value && self.read_only == other.read_only
    }
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
            let content = std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .unwrap();
            serde_yaml::from_str(&content)
                .map_err(|e| e.to_string())
                .ok()
        } else {
            None
        }
    }

    pub fn get_all() -> Vec<Self> {
        let path = default_credentials_location();
        let mut credentials = Vec::new();
        for entry in std::fs::read_dir(path).map_err(|e| e.to_string()).unwrap() {
            let entry = entry.map_err(|e| e.to_string()).unwrap();
            let path = entry.path();
            let credential = Credential::try_from(path)
                .map_err(|e| e.to_string())
                .unwrap();
            credentials.push(credential);
        }
        credentials
    }

    pub fn sync(&self) {
        let existing_credential = Credential::get(self.id.as_str());
        if let Some(existing_credential) = existing_credential {
            if existing_credential != *self {
                eprintln!("Updated credential {:?}", self.id);
                self.save();
            } else {
                eprintln!("Existing credential {:?}", self.id);
            }
        } else {
            eprintln!("New credential {:?}", self.id);
            self.save();
        }
    }

    fn save(&self) {
        let path = default_credentials_location().join(format!("{}.yml", self.id));
        let file = std::fs::File::create(&path)
            .map_err(|e| e.to_string())
            .unwrap();
        let writer = std::io::BufWriter::new(file);
        serde_yaml::to_writer(writer, self)
            .map_err(|e| e.to_string())
            .unwrap();
    }

    pub fn delete(&self) {
        let path = default_credentials_location().join(format!("{}.yml", self.id));
        std::fs::remove_file(&path)
            .map_err(|e| e.to_string())
            .unwrap();
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
        let file = std::fs::File::open(&path).map_err(|_| "Could not open file")?;
        let reader = std::io::BufReader::new(file);
        let yaml: serde_yaml::Value = serde_yaml::from_reader(reader).map_err(|e| e.to_string())?;

        let yaml_credential: YamlCredential =
            serde_yaml::from_value(yaml).map_err(|e| e.to_string())?;

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

pub fn default_credentials_location() -> PathBuf {
    let path = if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").map_err(|e| e.to_string()).unwrap();
        PathBuf::from(appdata).join("nomos").join("credentials")
    } else {
        PathBuf::from("/var/lib/nomos/credentials")
    };
    std::fs::create_dir_all(&path)
        .map_err(|e| e.to_string())
        .unwrap();
    path
}
