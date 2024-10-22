use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::{fs::File, io::BufReader, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::utils::{execute_command, execute_script};

pub trait ScriptExecutor {
    fn execute(
        &self,
        parameters: HashMap<String, String>,
        directory: PathBuf,
    ) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct BashScript {
    pub code: String,
}

#[derive(Debug, Clone)]
pub struct PythonScript {
    pub code: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitCloneScript {
    pub url: String,
    pub credential_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ScriptType {
    Bash(BashScript),
    Python(PythonScript),
    GitClone(GitCloneScript),
}

#[derive(Deserialize, Debug)]
pub struct ScriptParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ScriptStep {
    pub name: String,
    pub values: Vec<ScriptType>,
    pub is_started: bool,
    pub is_finished: bool,
    pub is_success: bool,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

pub struct Script {
    pub id: String,
    pub name: String,
    pub parameters: Vec<ScriptParameter>,
    pub steps: Vec<ScriptStep>,
}
impl Script {
    pub(crate) fn get(script_id: &str) -> Option<Self> {
        let path = default_scripts_location().join(format!("{}.yml", script_id));
        if path.exists() {
            let yaml_script = YamlScript::try_from(path).ok()?;
            Some(Script::from(yaml_script))
        } else {
            None
        }
    }

    pub fn get_from_path(path_str: &str) -> Option<Self> {
        let path = PathBuf::from(path_str);
        if path.exists() {
            let yaml_script = YamlScript::try_from(path).ok()?;
            Some(Script::from(yaml_script))
        } else {
            None
        }
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

#[derive(Deserialize, Debug)]
pub struct YamlScriptStep {
    pub name: String,
    pub bash: Option<String>,
    pub python: Option<String>,
    pub git_clone: Option<GitCloneScript>,
}

impl Default for YamlScriptStep {
    fn default() -> Self {
        YamlScriptStep {
            name: String::new(),
            bash: None,
            python: None,
            git_clone: None,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct YamlScript {
    pub id: String,
    pub name: String,
    pub parameters: Vec<ScriptParameter>,
    pub steps: Vec<YamlScriptStep>,
}

impl From<YamlScript> for Script {
    fn from(yaml_script: YamlScript) -> Self {
        let mut steps = vec![];
        for yaml_step in yaml_script.steps.iter() {
            let mut values = vec![];
            if let Some(bash) = &yaml_step.bash {
                values.push(ScriptType::Bash(BashScript {
                    code: bash.to_string(),
                }));
            }
            if let Some(python) = &yaml_step.python {
                values.push(ScriptType::Python(PythonScript {
                    code: python.to_string(),
                }));
            }
            if let Some(git_clone) = &yaml_step.git_clone {
                values.push(ScriptType::GitClone(GitCloneScript {
                    url: git_clone.url.to_string(),
                    credential_id: git_clone.credential_id.clone(),
                }));
            }
            steps.push(ScriptStep {
                name: yaml_step.name.to_string(),
                values,
                ..Default::default()
            });
        }
        Script {
            id: yaml_script.id.to_string(),
            name: yaml_script.name.to_string(),
            parameters: yaml_script.parameters,
            steps,
        }
    }
}

impl TryFrom<PathBuf> for YamlScript {
    type Error = &'static str;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let file = File::open(path).map_err(|_| "Could not open file")?;
        let reader = BufReader::new(file);
        serde_yaml::from_reader(reader).map_err(|_| "Could not parse YAML")
    }
}

impl ScriptExecutor for BashScript {
    fn execute(
        &self,
        parameters: HashMap<String, String>,
        directory: PathBuf,
    ) -> Result<(), String> {
        let mut replaced_code = self.code.clone();
        // example: $parameters.git_clone_url
        for (key, value) in parameters.iter() {
            replaced_code = replaced_code.replace(&format!("$parameters.{}", key), value);
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

impl ScriptExecutor for PythonScript {
    fn execute(
        &self,
        parameters: HashMap<String, String>,
        directory: PathBuf,
    ) -> Result<(), String> {
        let child = Command::new("python")
            .arg("-c")
            .arg(&self.code)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;

        execute_script(child)
    }
}

impl ScriptExecutor for GitCloneScript {
    fn execute(
        &self,
        parameters: HashMap<String, String>,
        directory: PathBuf,
    ) -> Result<(), String> {
        let mut url = self.url.clone();
        let is_variable = url.starts_with("$parameters.");
        if is_variable {
            let key = url.replace("$parameters.", "");
            url = parameters.get(&key).cloned().unwrap_or_default();
        }

        let mut credential_id = self.credential_id.clone();
        let is_variable = credential_id.as_ref().map_or(false, |id| id.starts_with("$parameters."));
        if is_variable {
            let key = credential_id.as_ref().unwrap().replace("$parameters.", "");
            credential_id = parameters.get(&key).cloned();
        }

        crate::git::git_clone(&url, directory, credential_id.as_deref())
            .map_err(|e| e.to_string())
    }
}

impl ScriptExecutor for ScriptType {
    fn execute(
        &self,
        parameters: HashMap<String, String>,
        directory: PathBuf,
    ) -> Result<(), String> {
        match self {
            ScriptType::Bash(bash) => bash.execute(parameters, directory),
            ScriptType::Python(python) => python.execute(parameters, directory),
            ScriptType::GitClone(git_clone) => git_clone.execute(parameters, directory),
        }
    }
}

impl ScriptExecutor for ScriptStep {
    fn execute(
        &self,
        parameters: HashMap<String, String>,
        directory: PathBuf,
    ) -> Result<(), String> {
        for value in self.values.iter() {
            value.execute(parameters.clone(), directory.clone())?;
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
