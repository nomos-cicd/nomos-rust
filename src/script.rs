use std::process::Command;
use std::{fs::File, io::BufReader, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::Deserialize;

pub trait ScriptExecutor {
    fn execute(&self) -> Result<(), String>;
}

pub struct BashScript {
    pub code: String,
}

pub struct PythonScript {
    pub code: String,
}

pub enum ScriptType {
    Bash(BashScript),
    Python(PythonScript),
}

#[derive(Deserialize, Debug)]
pub struct ScriptParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

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
            is_success: false,
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
    fn execute(&self) -> Result<(), String> {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(&["/C", &self.code])
                .output()
                .map_err(|e| e.to_string())?
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(&self.code)
                .output()
                .map_err(|e| e.to_string())?
        };
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }
}

impl ScriptExecutor for PythonScript {
    fn execute(&self) -> Result<(), String> {
        let output = Command::new("python")
            .arg("-c")
            .arg(&self.code)
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }
}

impl ScriptExecutor for ScriptType {
    fn execute(&self) -> Result<(), String> {
        match self {
            ScriptType::Bash(bash) => bash.execute(),
            ScriptType::Python(python) => python.execute(),
        }
    }
}

// impl Script {
//     pub fn execute(&self) -> Result<(), String> {
//         for step in self.steps.iter() {
//             for value in step.values.iter() {
//                 value.execute()?;
//             }
//         }
//         Ok(())
//     }
// }
