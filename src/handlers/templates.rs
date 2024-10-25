use askama::Template;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Html,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::{
    credential::{Credential, CredentialType},
    job::{self, JobResult},
    log::LogLevel,
    script::models::Script,
};

#[derive(Template)]
#[template(path = "credentials.html")]
struct CredentialsTemplate<'a> {
    title: &'a str,
    credentials: Vec<Credential>,
}

pub async fn template_credentials() -> Html<String> {
    let credentials = Credential::get_all().unwrap();
    let template = CredentialsTemplate {
        title: "Credentials",
        credentials,
    };
    Html(template.render().unwrap())
}

#[derive(Template)]
#[template(path = "credential.html")]
struct CredentialTemplate<'a> {
    title: &'a str,
    credential: Option<&'a Credential>,
    credential_value: &'a CredentialType,
}

pub async fn template_update_credential(Path(id): Path<String>) -> Html<String> {
    template_credential(Some(axum::extract::Path(id)), "Credentials").await
}

pub async fn template_create_credential() -> Html<String> {
    template_credential(None, "Create Credentials").await
}

pub async fn template_credential(id: Option<Path<String>>, title: &str) -> Html<String> {
    let credential = if let Some(id) = id {
        Credential::get(id.as_str())
    } else {
        None
    };
    let default_value = CredentialType::from_str("ssh").unwrap();

    let credential_value = if let Some(cred) = credential.as_ref() {
        &cred.value
    } else {
        &default_value
    };

    let template = CredentialTemplate {
        title,
        credential: credential.as_ref(),
        credential_value,
    };
    Html(template.render().unwrap())
}

#[derive(Template)]
#[template(path = "credential-value.html")]
struct CredentialValueTemplate<'a> {
    credential_value: &'a CredentialType,
}

#[derive(Deserialize)]
pub struct CredentialValueQuery {
    id: Option<String>,
    #[serde(rename = "type")]
    credential_type: String,
}

pub async fn template_credential_value(params: Query<CredentialValueQuery>) -> (StatusCode, Html<String>) {
    if let Some(id) = &params.id {
        let credential = Credential::get(id.as_str());
        if let Some(credential) = credential {
            if credential.get_credential_type() == params.credential_type {
                let template = CredentialValueTemplate {
                    credential_value: &credential.value,
                };
                return (StatusCode::OK, Html(template.render().unwrap()));
            }
        }
    }

    let credential_type = CredentialType::from_str(params.credential_type.as_str());
    if credential_type.is_ok() {
        let credential_type = credential_type.unwrap();
        let template = CredentialValueTemplate {
            credential_value: &credential_type,
        };
        return (StatusCode::OK, Html(template.render().unwrap()));
    }

    (StatusCode::BAD_REQUEST, Html("Invalid credential type".to_string()))
}

#[derive(Template)]
#[template(path = "scripts.html")]
struct ScriptsTemplate<'a> {
    title: &'a str,
    scripts: Vec<Script>,
}

#[derive(Template)]
#[template(path = "script.html")]
struct ScriptTemplate<'a> {
    title: &'a str,
    script: Option<&'a str>,
    // json_schema: &'a str,
}

pub async fn template_scripts() -> Html<String> {
    let scripts = Script::get_all().unwrap();
    let template = ScriptsTemplate {
        title: "Scripts",
        scripts,
    };
    Html(template.render().unwrap())
}

pub async fn template_update_script(Path(id): Path<String>) -> Html<String> {
    template_script(Some(axum::extract::Path(id)), "Scripts").await
}

pub async fn template_create_script() -> Html<String> {
    template_script(None, "Create Script").await
}

pub async fn template_script(id: Option<Path<String>>, title: &str) -> Html<String> {
    let script = if let Some(id) = id {
        Script::get(id.as_str())
    } else {
        None
    };

    let mut script_yaml = None;
    if let Some(script) = script.as_ref() {
        script_yaml = Some(serde_yaml::to_string(script).unwrap());
    }

    // let json_schema = Script::get_json_schema();
    // let json_schema_str = serde_json::to_string(&json_schema).unwrap();

    let template = ScriptTemplate {
        title,
        script: script_yaml.as_deref(),
        // json_schema: &json_schema_str,
    };
    Html(template.render().unwrap())
}

#[derive(Template)]
#[template(path = "jobs.html")]
struct JobsTemplate<'a> {
    title: &'a str,
    jobs: Vec<job::Job>,
}

#[derive(Template)]
#[template(path = "job.html")]
struct JobTemplate<'a> {
    title: &'a str,
    job: Option<&'a str>,
    // json_schema: &'a str,
}

pub async fn template_jobs() -> Html<String> {
    let jobs = job::Job::get_all().unwrap();
    let template = JobsTemplate { title: "Jobs", jobs };
    Html(template.render().unwrap())
}

pub async fn template_update_job(Path(id): Path<String>) -> Html<String> {
    template_job(Some(axum::extract::Path(id)), "Jobs").await
}

pub async fn template_create_job() -> Html<String> {
    template_job(None, "Create Job").await
}

pub async fn template_job(id: Option<Path<String>>, title: &str) -> Html<String> {
    let job = if let Some(id) = id {
        job::Job::get(id.as_str())
    } else {
        None
    };

    let mut job_yaml = None;
    if let Some(job) = job.as_ref() {
        job_yaml = Some(serde_yaml::to_string(job).unwrap());
    }

    // let json_schema = job::Job::get_json_schema();
    // let json_schema_str = serde_json::to_string(&json_schema).unwrap();

    let template = JobTemplate {
        title,
        job: job_yaml.as_deref(),
        // json_schema: &json_schema_str,
    };
    Html(template.render().unwrap())
}

#[derive(Template)]
#[template(path = "job-results.html")]
struct JobResultsTemplate<'a> {
    title: &'a str,
    results: Vec<JobResult>,
}

#[derive(Template)]
#[template(path = "job-result.html")]
struct JobResultTemplate<'a> {
    title: &'a str,
    result: &'a JobResult,
    now: DateTime<Utc>,
}

#[derive(Template)]
#[template(path = "job-result-logs.html")]
struct JobResultLogsTemplate<'a> {
    logs: Vec<FormattedLog<'a>>,
}

struct FormattedLog<'a> {
    timestamp: &'a DateTime<Utc>,
    level: &'a LogLevel,
    message: &'a str,
}

pub async fn template_job_results() -> Html<String> {
    let results = JobResult::get_all().unwrap();
    let template = JobResultsTemplate {
        title: "Job Results",
        results,
    };
    Html(template.render().unwrap())
}

pub async fn template_job_result(Path(id): Path<String>) -> (StatusCode, Html<String>) {
    let result = JobResult::get(&id);
    if result.is_none() {
        return (StatusCode::NOT_FOUND, Html("".to_string()));
    }
    let result = result.unwrap();
    let now = Utc::now();
    let template = JobResultTemplate {
        title: "Job Result",
        result: &result,
        now,
    };
    (StatusCode::OK, Html(template.render().unwrap()))
}

pub async fn template_job_result_logs(Path(result_id): Path<String>) -> Html<String> {
    let result = JobResult::get(&result_id).unwrap();
    let logs = result.logger.get_logs().unwrap();

    let formatted_logs: Vec<FormattedLog> = logs
        .iter()
        .map(|log| FormattedLog {
            timestamp: &log.timestamp,
            level: &log.level,
            message: &log.message,
        })
        .collect();

    let template = JobResultLogsTemplate { logs: formatted_logs };
    Html(template.render().unwrap())
}
