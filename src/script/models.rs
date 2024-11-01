use std::{fmt::Display, fs::File, io::BufReader, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{job::JobResult, log::LogLevel};

use super::{default_scripts_location, types::ScriptType, ScriptParameter};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum ScriptStatus {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "aborted")]
    Aborted,
}

impl Display for ScriptStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptStatus::Success => write!(f, "Success"),
            ScriptStatus::Failed => write!(f, "Failed"),
            ScriptStatus::Aborted => write!(f, "Aborted"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Script {
    pub id: String,
    pub name: String,
    pub parameters: Vec<ScriptParameter>,
    pub steps: Vec<ScriptStep>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct RunningScriptStep {
    pub name: String,
    pub values: Vec<ScriptType>,
    pub status: ScriptStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ScriptStep {
    pub name: String,
    pub values: Vec<ScriptType>,
}

impl Script {
    /// Reads as YamlScript and converts to Script. Primarily used before executing a job.
    pub(crate) fn get(script_id: &str) -> Result<Option<Self>, String> {
        let path = default_scripts_location()?.join(format!("{}.yml", script_id));
        if path.exists() {
            let yaml_script = Script::try_from(path)?;
            Ok(Some(yaml_script))
        } else {
            Ok(None)
        }
    }

    pub fn get_all() -> Result<Vec<Self>, String> {
        let scripts_path = default_scripts_location()?;
        let mut scripts = vec![];
        for entry in std::fs::read_dir(scripts_path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path: PathBuf = entry.path();
            match Script::try_from(path) {
                Ok(script) => scripts.push(script),
                Err(e) => eprintln!("Error reading script: {:?}", e),
            }
        }
        Ok(scripts)
    }

    /// Save as YamlScript. Primarily used after creating a new script.
    pub fn sync(&self, job_result: Option<&mut JobResult>) -> Result<(), String> {
        let existing_script = Script::get(self.id.as_str())?;

        if let Some(existing_script) = existing_script {
            if existing_script != *self {
                self.save()?;
                if let Some(job_result) = job_result {
                    job_result.add_log(LogLevel::Info, format!("Updated script {:?}", self.id))
                }
            } else if let Some(job_result) = job_result {
                job_result.add_log(LogLevel::Info, format!("No changes in script {:?}", self.id))
            }
        } else {
            self.save()?;
            if let Some(job_result) = job_result {
                job_result.add_log(LogLevel::Info, format!("Created script {:?}", self.id))
            }
        }

        Ok(())
    }

    fn save(&self) -> Result<(), String> {
        let path = default_scripts_location()?.join(format!("{}.yml", self.id));
        let file = File::create(path).map_err(|e| e.to_string())?;
        serde_yaml::to_writer(file, self).map_err(|e| e.to_string())
    }

    pub fn delete(&self) -> Result<(), String> {
        let path = default_scripts_location()?.join(format!("{}.yml", self.id));
        std::fs::remove_file(path).map_err(|e| e.to_string())
    }
}

impl TryFrom<PathBuf> for Script {
    type Error = &'static str;

    /// Reads as YamlScript and converts to Script. Primarily used for creating a new script.
    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let file = File::open(path).map_err(|_| "Could not open file")?;
        let reader = BufReader::new(file);
        serde_yaml::from_reader(reader).map_err(|e| {
            eprintln!("Error reading YAML: {}", e);
            "Could not parse YAML"
        })
    }
}

impl RunningScriptStep {
    pub fn start(&mut self) {
        self.started_at = Some(Utc::now());
    }

    pub fn finish(&mut self, status: ScriptStatus) {
        self.status = status;
        self.finished_at = Some(Utc::now());
    }
}

impl Default for RunningScriptStep {
    fn default() -> Self {
        RunningScriptStep {
            name: String::new(),
            values: vec![],
            status: ScriptStatus::Failed,
            started_at: None,
            finished_at: None,
        }
    }
}

impl From<&ScriptStep> for RunningScriptStep {
    fn from(step: &ScriptStep) -> Self {
        RunningScriptStep {
            name: step.name.clone(),
            values: step.values.clone(),
            ..Default::default()
        }
    }
}
