use chrono::{DateTime, Utc};

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

// pub fn run_script(script: &Script) {
//     for step in script.steps.iter() {
//         println!("Running step: {}", step.name);
//     }
// }

pub struct YamlScriptStep {
    pub name: String,
    pub bash: Option<String>,
    pub python: Option<String>,
}

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
