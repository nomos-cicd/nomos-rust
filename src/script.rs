use std::collections::HashMap;
use std::io::BufRead;
use std::process::{Child, Command, Stdio};
use std::{fs::File, io::BufReader, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::Deserialize;

pub trait ScriptExecutor {
    fn execute(&self, parameters: HashMap<String, String>) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct BashScript {
    pub code: String,
}

#[derive(Debug, Clone)]
pub struct PythonScript {
    pub code: String,
}

#[derive(Debug, Clone)]
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
    fn execute(&self, parameters: HashMap<String, String>) -> Result<(), String> {
        let child = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(&["/C", &self.code])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| e.to_string())?
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(&self.code)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| e.to_string())?
        };

        execute_script(child)
    }
}

impl ScriptExecutor for PythonScript {
    fn execute(&self, parameters: HashMap<String, String>) -> Result<(), String> {
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

impl ScriptExecutor for ScriptType {
    fn execute(&self, parameters: HashMap<String, String>) -> Result<(), String> {
        match self {
            ScriptType::Bash(bash) => bash.execute(parameters),
            ScriptType::Python(python) => python.execute(parameters),
        }
    }
}

impl ScriptExecutor for ScriptStep {
    fn execute(&self, parameters: HashMap<String, String>) -> Result<(), String> {
        for value in self.values.iter() {
            value.execute(parameters.clone())?;
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

fn execute_script(mut child: Child) -> Result<(), String> {
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    // Log stdout in real-time
    std::thread::spawn(move || {
        for line in stdout_reader.lines() {
            if let Ok(line) = line {
                println!("STDOUT: {}", line);
            }
        }
    });

    // Log stderr in real-time
    std::thread::spawn(move || {
        for line in stderr_reader.lines() {
            if let Ok(line) = line {
                eprintln!("STDERR: {}", line);
            }
        }
    });

    // Wait for the child process to finish
    let status = child.wait().map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Process exited with status: {}", status))
    }
}
