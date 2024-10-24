mod credential;
mod git;
mod job;
mod log;
mod script;
mod settings;
mod utils;

use askama::Template;
use axum::{
    extract::{Path, Query},
    http::{HeaderMap, StatusCode},
    response::Html,
    routing, Json, Router,
};
use chrono::{DateTime, Utc};
use credential::{Credential, CredentialType, YamlCredential};
use job::JobResult;
use log::LogLevel;
use script::Script;
use serde::Deserialize;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // build our application with a route
    let app = Router::new()
        .route("/api/credentials", routing::get(get_credentials))
        .route("/api/credentials/:id", routing::get(get_credential))
        .route("/api/credentials", routing::post(create_credential))
        .route("/api/credentials/:id", routing::delete(delete_credential))
        .route("/api/credential-types", routing::get(get_credential_types))
        .route("/api/scripts", routing::get(get_scripts))
        .route("/api/scripts/:id", routing::get(get_script))
        .route("/api/scripts", routing::post(create_script))
        .route("/api/scripts/:id", routing::delete(delete_script))
        .route("/script-parameter-types", routing::get(get_script_parameter_types))
        .route("/api/jobs", routing::get(get_jobs))
        .route("/api/jobs/:id", routing::get(get_job))
        .route("/api/jobs", routing::post(create_job))
        .route("/api/jobs/:id", routing::delete(delete_job))
        .route("/api/jobs/:id/execute", routing::post(execute_job))
        .route("/api/job-trigger-types", routing::get(get_job_trigger_types))
        .route("/api/job-results", routing::get(get_job_results))
        .route("/api/job-results/:id", routing::get(get_job_result))
        .route("/", routing::get(template_credentials))
        .route("/credentials", routing::get(template_credentials))
        .route("/credentials/create", routing::get(template_create_credential))
        .route("/credentials/:id", routing::get(template_update_credential))
        .route("/template/credential-value", routing::get(template_credential_value))
        .route("/scripts", routing::get(template_scripts))
        .route("/scripts/create", routing::get(template_create_script))
        .route("/scripts/:id", routing::get(template_update_script))
        .route("/jobs", routing::get(template_jobs))
        .route("/jobs/create", routing::get(template_create_job))
        .route("/jobs/:id", routing::get(template_update_job))
        .route("/job-results", routing::get(template_job_results))
        .route("/job-results/:id", routing::get(template_job_result))
        .route("/job-results/:id/logs", routing::get(template_job_result_logs))
        .layer(CorsLayer::permissive());

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_credentials() -> Json<Vec<credential::Credential>> {
    let credentials = credential::Credential::get_all().unwrap();
    Json(credentials)
}

async fn get_credential(Path(id): Path<String>) -> (StatusCode, Json<credential::Credential>) {
    let credential = credential::Credential::get(id.as_str());
    if credential.is_none() {
        return (StatusCode::NOT_FOUND, Json(credential.unwrap()));
    }

    (StatusCode::OK, Json(credential.unwrap()))
}

async fn create_credential(Json(credential): Json<YamlCredential>) -> Json<credential::Credential> {
    let credential = credential::Credential::try_from(credential).unwrap();
    credential.sync(None);
    Json(credential)
}

async fn delete_credential(Path(id): Path<String>) -> StatusCode {
    let credential = credential::Credential::get(id.as_str());
    if credential.is_none() {
        return StatusCode::NOT_FOUND;
    }
    credential.unwrap().delete();
    StatusCode::NO_CONTENT
}

async fn get_credential_types() -> Json<serde_json::Value> {
    let credential_types = credential::CredentialType::get_json_schema();
    Json(credential_types)
}

async fn get_scripts() -> Json<Vec<script::Script>> {
    let scripts = script::Script::get_all().unwrap();
    Json(scripts)
}

async fn get_script(Path(id): Path<String>) -> (StatusCode, Json<script::Script>) {
    let script = script::Script::get(id.as_str());
    if script.is_none() {
        return (StatusCode::NOT_FOUND, Json(script.unwrap()));
    }

    (StatusCode::OK, Json(script.unwrap()))
}

async fn create_script(headers: HeaderMap, body: String) -> (StatusCode, Json<script::Script>) {
    let content_type = headers.get("content-type");
    if content_type.is_none() {
        return (StatusCode::BAD_REQUEST, Json(script::Script::default()));
    }

    let content_type = content_type.unwrap().to_str().unwrap();
    if content_type != "application/yaml" {
        return (StatusCode::BAD_REQUEST, Json(script::Script::default()));
    }

    let script: Script = serde_yaml::from_str(body.as_str()).unwrap();
    script.sync(None);
    (StatusCode::CREATED, Json(script))
}

async fn delete_script(Path(id): Path<String>) -> StatusCode {
    let script = script::Script::get(id.as_str());
    if script.is_none() {
        return StatusCode::NOT_FOUND;
    }
    script.unwrap().delete();
    StatusCode::NO_CONTENT
}

async fn get_script_parameter_types() -> Json<serde_json::Value> {
    let script_parameter_types = script::ScriptParameterType::get_json_schema();
    Json(script_parameter_types)
}

async fn get_jobs() -> Json<Vec<job::Job>> {
    let jobs = job::Job::get_all().unwrap();
    Json(jobs)
}

async fn get_job(Path(id): Path<String>) -> (StatusCode, Json<job::Job>) {
    let job = job::Job::get(id.as_str());
    if job.is_none() {
        return (StatusCode::NOT_FOUND, Json(job.unwrap()));
    }

    (StatusCode::OK, Json(job.unwrap()))
}

async fn create_job(headers: HeaderMap, body: String) -> (StatusCode, Json<job::Job>) {
    let content_type = headers.get("content-type");
    if content_type.is_none() {
        return (StatusCode::BAD_REQUEST, Json(job::Job::default()));
    }

    let content_type = content_type.unwrap().to_str().unwrap();
    if content_type != "application/yaml" {
        return (StatusCode::BAD_REQUEST, Json(job::Job::default()));
    }

    let job: job::Job = serde_yaml::from_str(body.as_str()).unwrap();
    job.sync(None);
    (StatusCode::CREATED, Json(job))
}

async fn delete_job(Path(id): Path<String>) -> StatusCode {
    let job = job::Job::get(id.as_str());
    if job.is_none() {
        return StatusCode::NOT_FOUND;
    }
    job.unwrap().delete();
    StatusCode::NO_CONTENT
}

async fn execute_job(
    Path(id): Path<String>,
    /*Json(parameters): Json<HashMap<String, ScriptParameterType>>,*/
) -> (StatusCode, String) {
    let job = job::Job::get(id.as_str());
    if job.is_none() {
        return (StatusCode::NOT_FOUND, "".to_string());
    }
    let result = job.unwrap().execute(Default::default());
    if result.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "".to_string());
    }

    (StatusCode::OK, result.unwrap())
}

async fn get_job_trigger_types() -> Json<serde_json::Value> {
    let job_trigger_types = job::TriggerType::get_json_schema();
    Json(job_trigger_types)
}

async fn get_job_results() -> Json<Vec<job::JobResult>> {
    let job_results = job::JobResult::get_all().unwrap();
    Json(job_results)
}

async fn get_job_result(Path(id): Path<String>) -> Json<job::JobResult> {
    let job_result = job::JobResult::get(id.as_str());
    Json(job_result.unwrap())
}

#[derive(Template)]
#[template(path = "credentials.html")]
struct CredentialsTemplate<'a> {
    title: &'a str,
    credentials: Vec<Credential>,
}

async fn template_credentials() -> Html<String> {
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

async fn template_update_credential(Path(id): Path<String>) -> Html<String> {
    template_credential(Some(axum::extract::Path(id)), "Credentials").await
}

async fn template_create_credential() -> Html<String> {
    template_credential(None, "Create Credentials").await
}

async fn template_credential(id: Option<Path<String>>, title: &str) -> Html<String> {
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
struct CredentialValueQuery {
    id: Option<String>,
    #[serde(rename = "type")]
    credential_type: String,
}

async fn template_credential_value(params: Query<CredentialValueQuery>) -> (StatusCode, Html<String>) {
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
    json_schema: &'a str,
}

async fn template_scripts() -> Html<String> {
    let scripts = Script::get_all().unwrap();
    let template = ScriptsTemplate {
        title: "Scripts",
        scripts,
    };
    Html(template.render().unwrap())
}

async fn template_update_script(Path(id): Path<String>) -> Html<String> {
    template_script(Some(axum::extract::Path(id)), "Scripts").await
}

async fn template_create_script() -> Html<String> {
    template_script(None, "Create Script").await
}

async fn template_script(id: Option<Path<String>>, title: &str) -> Html<String> {
    let script = if let Some(id) = id {
        Script::get(id.as_str())
    } else {
        None
    };

    let mut script_yaml = None;
    if let Some(script) = script.as_ref() {
        script_yaml = Some(serde_yaml::to_string(script).unwrap());
    }

    let json_schema = script::Script::get_json_schema();
    let json_schema_str = serde_json::to_string(&json_schema).unwrap();

    let template = ScriptTemplate {
        title,
        script: script_yaml.as_deref(),
        json_schema: &json_schema_str,
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
    json_schema: &'a str,
}

async fn template_jobs() -> Html<String> {
    let jobs = job::Job::get_all().unwrap();
    let template = JobsTemplate { title: "Jobs", jobs };
    Html(template.render().unwrap())
}

async fn template_update_job(Path(id): Path<String>) -> Html<String> {
    template_job(Some(axum::extract::Path(id)), "Jobs").await
}

async fn template_create_job() -> Html<String> {
    template_job(None, "Create Job").await
}

async fn template_job(id: Option<Path<String>>, title: &str) -> Html<String> {
    let job = if let Some(id) = id {
        job::Job::get(id.as_str())
    } else {
        None
    };

    let mut job_yaml = None;
    if let Some(job) = job.as_ref() {
        job_yaml = Some(serde_yaml::to_string(job).unwrap());
    }

    let json_schema = job::Job::get_json_schema();
    let json_schema_str = serde_json::to_string(&json_schema).unwrap();

    let template = JobTemplate {
        title,
        job: job_yaml.as_deref(),
        json_schema: &json_schema_str,
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

async fn template_job_results() -> Html<String> {
    let results = JobResult::get_all().unwrap();
    let template = JobResultsTemplate {
        title: "Job Results",
        results,
    };
    Html(template.render().unwrap())
}

async fn template_job_result(Path(id): Path<String>) -> Html<String> {
    let result = JobResult::get(&id).unwrap();
    let now = Utc::now();
    let template = JobResultTemplate {
        title: "Job Result",
        result: &result,
        now,
    };
    Html(template.render().unwrap())
}

async fn template_job_result_logs(Path(result_id): Path<String>) -> Html<String> {
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
