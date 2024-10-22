use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::{fs::File, io::BufReader, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::git::git_clone;
use crate::utils::{execute_command, execute_script};

pub trait ScriptExecutor {
    fn execute(
        &self,
        parameters: &mut HashMap<String, String>,
        directory: PathBuf,
        step_name: &str,
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
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum ScriptType {
    #[serde(rename = "bash")]
    Bash(BashScript),
    #[serde(rename = "git-clone")]
    GitClone(GitCloneScript),
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
pub struct ScriptParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ScriptStep {
    pub name: String,
    pub values: Vec<ScriptType>,
    pub is_started: bool,
    pub is_finished: bool,
    pub is_success: bool,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct Script {
    pub id: String,
    pub name: String,
    pub parameters: Vec<ScriptParameter>,
    pub steps: Vec<ScriptStep>,
}

impl Script {
    /// Reads as YamlScript and converts to Script. Primarily used before executing a job.
    pub(crate) fn get(script_id: &str) -> Option<Self> {
        let path = default_scripts_location().join(format!("{}.yml", script_id));
        if path.exists() {
            let yaml_script = YamlScript::try_from(path).ok()?;
            Some(Script::from(yaml_script))
        } else {
            None
        }
    }

    /// Save as YamlScript. Primarily used after creating a new script.
    pub fn sync(&self) {
        let existing_script = Script::get(self.id.as_str());
        if let Some(existing_script) = existing_script {
            if existing_script.name != self.name
                || existing_script.parameters != self.parameters
                || existing_script.steps != self.steps
            {
                self.save();
            }
        } else {
            self.save();
        }
    }

    fn save(&self) {
        eprintln!("Saving script: {}", self.id);
        let path = default_scripts_location().join(format!("{}.yml", self.id));
        let file = File::create(path).expect("Could not create file");
        serde_yaml::to_writer(file, &YamlScript::from(self)).expect("Could not write to file");
    }
}

impl TryFrom<PathBuf> for Script {
    type Error = &'static str;

    /// Reads as YamlScript and converts to Script. Primarily used for creating a new script.
    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let yaml_script = YamlScript::try_from(path).map_err(|_| "Could not parse YAML")?;
        Ok(Script::from(yaml_script))
    }
}

impl Default for ScriptParameter {
    fn default() -> Self {
        ScriptParameter {
            name: String::new(),
            description: String::new(),
            required: false,
            default: None,
        }
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

#[derive(Serialize, Deserialize, Debug)]
pub struct YamlScriptStep {
    pub name: String,
    pub values: Vec<ScriptType>,
}

impl Default for YamlScriptStep {
    fn default() -> Self {
        YamlScriptStep {
            name: String::new(),
            values: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct YamlScript {
    pub id: String,
    pub name: String,
    pub parameters: Vec<ScriptParameter>,
    pub steps: Vec<YamlScriptStep>,
}

impl From<YamlScript> for Script {
    fn from(yaml_script: YamlScript) -> Self {
        let steps = yaml_script
            .steps
            .iter()
            .map(|step| ScriptStep {
                name: step.name.clone(),
                values: step.values.clone(),
                ..Default::default()
            })
            .collect();
        Script {
            id: yaml_script.id,
            name: yaml_script.name,
            parameters: yaml_script.parameters,
            steps,
        }
    }
}

impl From<&Script> for YamlScript {
    fn from(script: &Script) -> Self {
        let steps = script
            .steps
            .iter()
            .map(|step| YamlScriptStep {
                name: step.name.clone(),
                values: step.values.clone(),
            })
            .collect();
        YamlScript {
            id: script.id.to_string(),
            name: script.name.to_string(),
            parameters: script.parameters.clone(),
            steps,
        }
    }
}

impl TryFrom<PathBuf> for YamlScript {
    type Error = &'static str;

    /// Reads as YamlScript and converts to Script. Primarily used before executing a job.
    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let file = File::open(path).map_err(|_| "Could not open file")?;
        let reader = BufReader::new(file);
        serde_yaml::from_reader(reader).map_err(|e| {
            eprintln!("Error reading YAML: {}", e);
            "Could not parse YAML"
        })
    }
}

impl ScriptExecutor for BashScript {
    fn execute(
        &self,
        parameters: &mut HashMap<String, String>,
        directory: PathBuf,
        step_name: &str,
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

        execute_command(&cmd_code, directory)
    }
}

impl ScriptExecutor for GitCloneScript {
    fn execute(
        &self,
        parameters: &mut HashMap<String, String>,
        directory: PathBuf,
        step_name: &str,
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

        git_clone(&url, directory.clone(), credential_id.as_deref()).map_err(|e| e.to_string())?;

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

impl ScriptExecutor for ScriptType {
    fn execute(
        &self,
        parameters: &mut HashMap<String, String>,
        directory: PathBuf,
        step_name: &str,
    ) -> Result<(), String> {
        match self {
            ScriptType::Bash(bash) => bash.execute(parameters, directory, step_name),
            ScriptType::GitClone(git_clone) => git_clone.execute(parameters, directory, step_name),
        }
    }
}

impl ScriptExecutor for ScriptStep {
    fn execute(
        &self,
        parameters: &mut HashMap<String, String>,
        directory: PathBuf,
        step_name: &str,
    ) -> Result<(), String> {
        for value in self.values.iter() {
            value.execute(parameters, directory.clone(), step_name)?;
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
        let appdata = std::env::var("APPDATA").expect("Could not get APPDATA");
        PathBuf::from(appdata).join("nomos").join("scripts")
    } else {
        PathBuf::from("/var/lib/nomos/scripts")
    };
    std::fs::create_dir_all(&path).expect("Could not create scripts directory");
    path
}
