use crate::script::ScriptStep;

use chrono::{DateTime, Utc};

struct ManualTriggerParameter {
}

struct GithubTriggerParameter {
    pub branch: String,
    pub events: Vec<String>,
    pub secret: String,
    pub url: String,
}

enum TriggerType {
    Manual(ManualTriggerParameter),
    Github(GithubTriggerParameter),
}

struct Trigger {
    pub value: TriggerType,
}

struct JobParameterDefinition {
    pub name: String,
    pub default: Option<String>,
}

struct Job {
    pub id: String,
    pub name: String,
    pub parameters: Vec<JobParameterDefinition>,
    pub triggers: Vec<Trigger>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

struct JobResult {
    pub id: String,
    pub job_id: String,
    pub is_finished: bool,
    pub is_success: bool,
    pub steps: Vec<ScriptStep>,
    pub current_step: Option<ScriptStep>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
