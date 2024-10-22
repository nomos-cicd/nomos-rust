use std::collections::HashMap;
use std::{fs::File, io::BufReader, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::git::git_clone;
use crate::job::JobResult;
use crate::utils::execute_command;
use crate::{log, settings};

pub trait ScriptExecutor {
    fn execute(
        &self,
        parameters: &mut HashMap<String, String>,
        directory: PathBuf,
        step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BashScript {
    pub code: String,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct GitCloneScript {
    pub url: String,
    pub credential_id: Option<String>,
    pub branch: Option<String>,
}

/// Scans directory for credential, script and job files and syncs them with the database.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SyncScript {
    pub directory: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum ScriptType {
    #[serde(rename = "bash")]
    Bash(BashScript),
    #[serde(rename = "git-clone")]
    GitClone(GitCloneScript),
    #[serde(rename = "sync")]
    Sync(SyncScript),
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Default, Debug)]
pub struct ScriptParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ScriptStep {
    pub name: String,
    pub values: Vec<ScriptType>,
    pub is_started: bool,
    pub is_finished: bool,
    pub is_success: bool,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Script {
    pub id: String,
    pub name: String,
    pub parameters: Vec<ScriptParameter>,
    pub steps: Vec<YamlScriptStep>,
}

impl Script {
    /// Reads as YamlScript and converts to Script. Primarily used before executing a job.
    pub(crate) fn get(script_id: &str) -> Option<Self> {
        let path = default_scripts_location().join(format!("{}.yml", script_id));
        if path.exists() {
            let yaml_script = Script::try_from(path).ok()?;
            Some(Script::from(yaml_script))
        } else {
            None
        }
    }

    pub fn get_all() -> Vec<Self> {
        let scripts_path = default_scripts_location();
        let mut scripts = vec![];
        for entry in std::fs::read_dir(scripts_path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let script = Script::try_from(path).unwrap();
            scripts.push(script);
        }
        scripts
    }

    /// Save as YamlScript. Primarily used after creating a new script.
    pub fn sync(&self, job_result: Option<&mut JobResult>) {
        let existing_script = Script::get(self.id.as_str());

        if let Some(existing_script) = existing_script {
            if existing_script != *self {
                self.save();
                job_result.map(|job_result| {
                    job_result.add_log(log::LogLevel::Info, format!("Updated script {:?}", self.id))
                });
            } else {
                job_result.map(|job_result| {
                    job_result.add_log(
                        log::LogLevel::Info,
                        format!("No changes in script {:?}", self.id),
                    )
                });
            }
        } else {
            self.save();
            job_result.map(|job_result| {
                job_result.add_log(log::LogLevel::Info, format!("Created script {:?}", self.id))
            });
        }
    }

    fn save(&self) {
        let path = default_scripts_location().join(format!("{}.yml", self.id));
        let file = File::create(path).map_err(|e| e.to_string()).unwrap();
        serde_yaml::to_writer(file, self)
            .map_err(|e| e.to_string())
            .unwrap();
    }

    pub fn delete(&self) {
        let path = default_scripts_location().join(format!("{}.yml", self.id));
        std::fs::remove_file(path)
            .map_err(|e| e.to_string())
            .unwrap();
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

impl Default for ScriptStep {
    fn default() -> Self {
        ScriptStep {
            name: String::new(),
            values: vec![],
            is_started: false,
            is_finished: false,
            is_success: true,
            started_at: Utc::now(),
            finished_at: Utc::now(),
        }
    }
}

#[derive(Serialize, Deserialize, Default, PartialEq, Debug)]
pub struct YamlScriptStep {
    pub name: String,
    pub values: Vec<ScriptType>,
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

impl ScriptExecutor for BashScript {
    fn execute(
        &self,
        parameters: &mut HashMap<String, String>,
        directory: PathBuf,
        _step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String> {
        let mut replaced_code = self.code.clone();
        for (key, value) in parameters.iter() {
            replaced_code = replaced_code.replace(key, value);
        }

        let mut cmd_code = String::new();
        for line in replaced_code.lines() {
            cmd_code.push_str(line);
            if cfg!(target_os = "windows") {
                cmd_code.push_str(" && ");
            } else {
                cmd_code.push_str(" ; ");
            }
        }

        if cmd_code.ends_with(" && ") {
            cmd_code = cmd_code[..cmd_code.len() - 4].to_string();
        } else if cmd_code.ends_with(" ; ") {
            cmd_code = cmd_code[..cmd_code.len() - 3].to_string();
        }

        execute_command(&cmd_code, directory, job_result)
    }
}

impl ScriptExecutor for GitCloneScript {
    fn execute(
        &self,
        parameters: &mut HashMap<String, String>,
        directory: PathBuf,
        step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String> {
        let mut url = self.url.clone();
        let is_variable = url.starts_with("$parameters.");
        if is_variable {
            url = parameters.get(&url).cloned().unwrap_or_default();
        }

        let mut credential_id = self.credential_id.clone();
        let is_variable = credential_id
            .as_ref()
            .map_or(false, |id| id.starts_with("$parameters."));
        if is_variable {
            credential_id = parameters.get(credential_id.as_ref().unwrap()).cloned();
        }

        let mut branch = self.branch.clone();
        let is_variable = branch
            .as_ref()
            .map_or(false, |b| b.starts_with("$parameters."));
        if is_variable {
            branch = branch.and_then(|b| parameters.get(&b).cloned());
        } else if branch.is_none() {
            branch = "main".to_string().into();
        }

        git_clone(
            &url,
            branch.unwrap().as_str(),
            directory.clone(),
            credential_id.as_deref(),
            job_result,
        )
        .map_err(|e| e.to_string())?;

        let mut cloned_dir = directory.clone().join(url.split('/').last().unwrap());
        if cloned_dir.to_str().unwrap().ends_with(".git") {
            cloned_dir = cloned_dir.parent().unwrap().to_path_buf();
        }

        parameters.insert(
            format!("$steps.{}.git-clone.directory", step_name),
            cloned_dir.to_str().unwrap().to_string(),
        );

        Ok(())
    }
}

impl ScriptExecutor for SyncScript {
    fn execute(
        &self,
        parameters: &mut HashMap<String, String>,
        directory: PathBuf,
        _step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String> {
        let is_variable = self.directory.starts_with('$');
        let mut param_directory = if is_variable {
            PathBuf::from(
                parameters
                    .get(&self.directory)
                    .cloned()
                    .expect("Could not get directory"),
            )
        } else {
            PathBuf::from(&self.directory)
        };

        if !param_directory.exists() {
            return Err(format!("Directory does not exist: {:?}", param_directory));
        }

        if param_directory.is_relative() {
            param_directory = directory.join(param_directory);
        }

        settings::sync(param_directory, job_result.into())
    }
}

impl ScriptExecutor for ScriptType {
    fn execute(
        &self,
        parameters: &mut HashMap<String, String>,
        directory: PathBuf,
        step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String> {
        match self {
            ScriptType::Bash(bash) => bash.execute(parameters, directory, step_name, job_result),
            ScriptType::GitClone(git_clone) => {
                git_clone.execute(parameters, directory, step_name, job_result)
            }
            ScriptType::Sync(sync) => sync.execute(parameters, directory, step_name, job_result),
        }
    }
}

impl ScriptExecutor for ScriptStep {
    fn execute(
        &self,
        parameters: &mut HashMap<String, String>,
        directory: PathBuf,
        step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String> {
        for value in self.values.iter() {
            value.execute(parameters, directory.clone(), step_name, job_result)?;
        }
        Ok(())
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

pub fn default_scripts_location() -> PathBuf {
    let path = if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").map_err(|e| e.to_string()).unwrap();
        PathBuf::from(appdata).join("nomos").join("scripts")
    } else {
        PathBuf::from("/var/lib/nomos/scripts")
    };
    std::fs::create_dir_all(&path)
        .map_err(|e| e.to_string())
        .unwrap();
    path
}
