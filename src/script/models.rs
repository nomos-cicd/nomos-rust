use std::{fs::File, io::BufReader, path::PathBuf};

use chrono::{DateTime, Utc};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};

use crate::{job::JobResult, log::LogLevel};

use super::{default_scripts_location, types::ScriptType, ScriptParameter};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, JsonSchema)]
pub struct Script {
    pub id: String,
    pub name: String,
    pub parameters: Vec<ScriptParameter>,
    pub steps: Vec<YamlScriptStep>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct ScriptStep {
    pub name: String,
    pub values: Vec<ScriptType>,
    pub is_started: bool,
    pub is_finished: bool,
    pub is_success: bool,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Default, PartialEq, JsonSchema, Clone, Debug)]
pub struct YamlScriptStep {
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
            let script = Script::try_from(path);
            if let Err(e) = script {
                eprintln!("Error reading script: {:?}", e);
                continue;
            }
            scripts.push(script.unwrap());
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

    #[allow(dead_code)]
    pub fn get_json_schema() -> Result<serde_json::Value, String> {
        let schema = schema_for!(Script);
        serde_json::to_value(schema).map_err(|e| e.to_string())
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

impl ScriptStep {
    pub fn start(&mut self) {
        self.is_started = true;
        self.started_at = Utc::now();
    }

    pub fn finish(&mut self, is_success: bool) {
        self.is_finished = true;
        self.is_success = is_success;
        self.finished_at = Utc::now();
    }
}

impl Default for ScriptStep {
    fn default() -> Self {
        ScriptStep {
            name: String::new(),
            values: vec![],
            is_started: false,
            is_finished: false,
            is_success: false,
            started_at: Utc::now(),
            finished_at: Utc::now(),
        }
    }
}

impl From<&YamlScriptStep> for ScriptStep {
    fn from(step: &YamlScriptStep) -> Self {
        ScriptStep {
            name: step.name.clone(),
            values: step.values.clone(),
            ..Default::default()
        }
    }
}
