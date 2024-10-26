use std::str::FromStr;

use askama::Template;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    Form,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::{
    credential::{Credential, CredentialType},
    job::{self, Job, JobResult},
    log::LogLevel,
    script::models::Script,
};

use super::{auth, Credentials};

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    next: Option<String>,
    title: String,
}

// This allows us to extract the "next" field from the query string. We use this
// to redirect after log in.
#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: Option<String>,
}

pub async fn template_get_login(Query(NextUrl { next }): Query<NextUrl>) -> Html<String> {
    let template = LoginTemplate {
        next,
        title: "Login".to_string(),
    };
    template.render().unwrap().into()
}

pub async fn template_post_login(
    mut auth_session: auth::AuthSession,
    Form(creds): Form<Credentials>,
) -> impl IntoResponse {
    let user = match auth_session.authenticate(creds.clone()).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            let mut login_url = "/login".to_string();
            if let Some(next) = creds.next {
                login_url = format!("{}?next={}", login_url, next);
            };

            return Redirect::to(&login_url).into_response();
        }
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if auth_session.login(&user).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    if let Some(ref next) = creds.next {
        Redirect::to(next)
    } else {
        Redirect::to("/")
    }
    .into_response()
}

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
        Credential::get(id.as_str(), None)
    } else {
        Ok(None)
    };
    if let Err(e) = credential {
        return Html(e.to_string());
    }
    let credential = credential.unwrap();
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
        let credential = Credential::get(id.as_str(), None);
        if let Err(e) = credential {
            return (StatusCode::INTERNAL_SERVER_ERROR, Html(e.to_string()));
        }
        let credential = credential.unwrap();
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

pub async fn template_scripts() -> (StatusCode, Html<String>) {
    let scripts = Script::get_all();
    if let Err(e) = scripts {
        return (StatusCode::INTERNAL_SERVER_ERROR, Html(e.to_string()));
    }
    let scripts = scripts.unwrap();
    let template = ScriptsTemplate {
        title: "Scripts",
        scripts,
    };
    (StatusCode::OK, Html(template.render().unwrap()))
}

pub async fn template_update_script(Path(id): Path<String>) -> (StatusCode, Html<String>) {
    template_script(Some(axum::extract::Path(id)), "Scripts").await
}

pub async fn template_create_script() -> (StatusCode, Html<String>) {
    template_script(None, "Create Script").await
}

pub async fn template_script(id: Option<Path<String>>, title: &str) -> (StatusCode, Html<String>) {
    let script = if let Some(id) = id {
        Script::get(id.as_str())
    } else {
        Ok(None)
    };
    if let Err(e) = script {
        return (StatusCode::INTERNAL_SERVER_ERROR, Html(e.to_string()));
    }
    let script = script.unwrap();

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
    (StatusCode::OK, Html(template.render().unwrap()))
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

#[derive(Deserialize, Default)]
pub struct JobFormQuery {
    #[serde(rename = "from-script-id")]
    from_script_id: Option<String>,
    #[serde(rename = "from-job-id")]
    from_job_id: Option<String>,
}

pub async fn template_jobs() -> Html<String> {
    let jobs = job::Job::get_all().unwrap();
    let template = JobsTemplate { title: "Jobs", jobs };
    Html(template.render().unwrap())
}

pub async fn template_update_job(Path(id): Path<String>) -> Html<String> {
    template_job(Some(axum::extract::Path(id)), "Jobs", Default::default()).await
}

pub async fn template_create_job(params: Query<JobFormQuery>) -> Html<String> {
    template_job(None, "Create Job", params).await
}

pub async fn template_job(id: Option<Path<String>>, title: &str, params: Query<JobFormQuery>) -> Html<String> {
    let job = if let Some(id) = id {
        job::Job::get(id.as_str())
    } else {
        Ok(None)
    };
    if let Err(e) = job {
        return Html(e.to_string());
    }
    let job = job.unwrap();

    let mut job_yaml = None;
    if let Some(job) = job.as_ref() {
        job_yaml = Some(serde_yaml::to_string(job).unwrap());
    }

    if let Some(from_script_id) = &params.from_script_id {
        let script = Script::get(from_script_id.as_str());
        if let Err(e) = script {
            return Html(e.to_string());
        }
        let script = script.unwrap();
        if let Some(script) = script {
            job_yaml = Some(serde_yaml::to_string(&Job::from(&script)).unwrap());
        }
    }

    if let Some(from_job_id) = &params.from_job_id {
        let job = job::Job::get(from_job_id.as_str());
        if let Err(e) = job {
            return Html(e.to_string());
        }
        let job = job.unwrap();
        if let Some(job) = job {
            job_yaml = Some(serde_yaml::to_string(&job).unwrap());
        }
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

#[derive(Deserialize)]
pub struct JobResultsQuery {
    #[serde(rename = "job-id")]
    job_id: Option<String>,
}
pub async fn template_job_results(query: Query<JobResultsQuery>) -> Html<String> {
    let results = JobResult::get_all(query.job_id.clone()).unwrap();
    let template = JobResultsTemplate {
        title: "Job Results",
        results,
    };
    Html(template.render().unwrap())
}

pub async fn template_job_result(Path(id): Path<String>) -> (StatusCode, Html<String>) {
    let result = JobResult::get(&id);
    if let Err(e) = result {
        return (StatusCode::INTERNAL_SERVER_ERROR, Html(e.to_string()));
    }
    let result = result.unwrap();
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

pub async fn template_job_result_logs(Path(result_id): Path<String>) -> (StatusCode, Html<String>) {
    let result = JobResult::get(&result_id);
    if let Err(e) = result {
        return (StatusCode::INTERNAL_SERVER_ERROR, Html(e.to_string()));
    }
    let result = result.unwrap();
    if result.is_none() {
        return (StatusCode::NOT_FOUND, Html("".to_string()));
    }
    let result = result.unwrap();
    if let Ok(logger) = result.logger.lock() {
        let logs = logger.get_logs().unwrap();

        let formatted_logs: Vec<FormattedLog> = logs
            .iter()
            .map(|log| FormattedLog {
                timestamp: &log.timestamp,
                level: &log.level,
                message: &log.message,
            })
            .collect();

        let template = JobResultLogsTemplate { logs: formatted_logs };
        return (StatusCode::OK, Html(template.render().unwrap()));
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Html("Failed to get logs".to_string()),
    )
}
