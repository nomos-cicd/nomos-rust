use std::collections::HashMap;

use axum::{
    extract::{Path, Query},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;

use crate::{
    credential::{self, YamlCredential},
    job,
    script::{self, models::Script},
    utils::is_signature_valid,
};

pub async fn get_credentials() -> Json<Vec<credential::Credential>> {
    let credentials = credential::Credential::get_all().unwrap();
    Json(credentials)
}

pub async fn get_credential(Path(id): Path<String>) -> (StatusCode, Json<credential::Credential>) {
    let credential = credential::Credential::get(id.as_str());
    if credential.is_none() {
        return (StatusCode::NOT_FOUND, Json(Default::default()));
    }

    (StatusCode::OK, Json(credential.unwrap()))
}

pub async fn create_credential(Json(credential): Json<YamlCredential>) -> Json<credential::Credential> {
    let credential = credential::Credential::try_from(credential).unwrap();
    credential.sync(None);
    Json(credential)
}

pub async fn delete_credential(Path(id): Path<String>) -> StatusCode {
    let credential = credential::Credential::get(id.as_str());
    if credential.is_none() {
        return StatusCode::NOT_FOUND;
    }
    credential.unwrap().delete();
    StatusCode::NO_CONTENT
}

pub async fn get_credential_types() -> Json<serde_json::Value> {
    let credential_types = credential::CredentialType::get_json_schema();
    Json(credential_types)
}

pub async fn get_scripts() -> Json<Vec<Script>> {
    let scripts = Script::get_all().unwrap();
    Json(scripts)
}

pub async fn get_script(Path(id): Path<String>) -> (StatusCode, Json<Script>) {
    let script = Script::get(id.as_str());
    if script.is_none() {
        return (StatusCode::NOT_FOUND, Json(Default::default()));
    }

    (StatusCode::OK, Json(script.unwrap()))
}

pub async fn create_script(headers: HeaderMap, body: String) -> (StatusCode, Json<Script>) {
    let content_type = headers.get("content-type");
    if content_type.is_none() {
        return (StatusCode::BAD_REQUEST, Json(Script::default()));
    }

    let content_type = content_type.unwrap().to_str().unwrap();
    if content_type != "application/yaml" {
        return (StatusCode::BAD_REQUEST, Json(Script::default()));
    }

    let script: Script = serde_yaml::from_str(body.as_str()).unwrap();
    script.sync(None);
    (StatusCode::CREATED, Json(script))
}

pub async fn delete_script(Path(id): Path<String>) -> StatusCode {
    let script = Script::get(id.as_str());
    if script.is_none() {
        return StatusCode::NOT_FOUND;
    }
    script.unwrap().delete();
    StatusCode::NO_CONTENT
}

pub async fn get_script_parameter_types() -> Json<serde_json::Value> {
    let script_parameter_types = script::ScriptParameterType::get_json_schema();
    Json(script_parameter_types)
}

pub async fn get_jobs() -> Json<Vec<job::Job>> {
    let jobs = job::Job::get_all().unwrap();
    Json(jobs)
}

pub async fn get_job(Path(id): Path<String>) -> (StatusCode, Json<job::Job>) {
    let job = job::Job::get(id.as_str());
    if job.is_none() {
        return (StatusCode::NOT_FOUND, Json(Default::default()));
    }

    (StatusCode::OK, Json(job.unwrap()))
}

pub async fn create_job(headers: HeaderMap, body: String) -> (StatusCode, Json<job::Job>) {
    let content_type = headers.get("content-type");
    if content_type.is_none() {
        return (StatusCode::BAD_REQUEST, Json(job::Job::default()));
    }

    let content_type = content_type.unwrap().to_str().unwrap();
    if content_type != "application/yaml" {
        return (StatusCode::BAD_REQUEST, Json(job::Job::default()));
    }

    let job: job::Job = serde_yaml::from_str(body.as_str()).unwrap();
    let res = job.sync(None);
    if res.is_err() {
        return (StatusCode::BAD_REQUEST, Json(job::Job::default()));
    }
    (StatusCode::CREATED, Json(job))
}

pub async fn delete_job(Path(id): Path<String>) -> StatusCode {
    let job = job::Job::get(id.as_str());
    if job.is_none() {
        return StatusCode::NOT_FOUND;
    }
    job.unwrap().delete();
    StatusCode::NO_CONTENT
}

pub async fn execute_job(
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

pub async fn dry_run_job(headers: HeaderMap, body: String) -> (StatusCode, String) {
    let content_type = headers.get("content-type");
    if content_type.is_none() {
        return (StatusCode::BAD_REQUEST, "Empty content-type".to_string());
    }

    let content_type = content_type.unwrap().to_str().unwrap();
    if content_type != "application/yaml" {
        return (
            StatusCode::BAD_REQUEST,
            "Only application/yaml is supported".to_string(),
        );
    }

    let job: job::Job = serde_yaml::from_str(body.as_str()).unwrap();
    let res = job.validate(None, Default::default());
    if res.is_err() {
        return (StatusCode::BAD_REQUEST, res.unwrap_err());
    }

    (StatusCode::OK, "".to_string())
}

pub async fn job_webhook_trigger(headers: HeaderMap, body: String) -> StatusCode {
    let jobs = job::Job::get_all();
    if jobs.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    let jobs = jobs.unwrap();
    for job in jobs {
        for trigger in job.triggers.iter() {
            match trigger {
                job::TriggerType::Github(val) => {
                    let signature = headers.get("x-hub-signature-256");
                    let github_event = headers.get("x-github-event");
                    if signature.is_none() || github_event.is_none() {
                        eprintln!("Signature or Event not found in headers");
                        continue;
                    }

                    let payload: Result<job::GithubPayload, serde_json::Error> = serde_json::from_str(body.as_str());
                    if payload.is_err() {
                        continue;
                    }
                    let payload = payload.unwrap();

                    let credential = credential::Credential::get(val.secret_credential_id.as_str());
                    if credential.is_none() {
                        eprintln!("Credential not found: {}", val.secret_credential_id);
                        continue;
                    }
                    let credential = credential.unwrap();
                    let text_credential: Option<credential::TextCredentialParameter> = match credential.value {
                        credential::CredentialType::Text(val) => Some(val),
                        _ => None,
                    };
                    if text_credential.is_none() {
                        eprintln!("Credential is not Text: {}", val.secret_credential_id);
                        continue;
                    }
                    let text_credential = text_credential.unwrap();

                    let is_valid =
                        is_signature_valid(&body, signature.unwrap().to_str().unwrap(), &text_credential.value);
                    if !is_valid {
                        eprintln!("Invalid signature");
                        continue;
                    }

                    if payload.repository.full_name != val.url {
                        eprintln!("Repository does not match");
                        continue;
                    }
                    if !val.events.iter().any(|x| x == github_event.unwrap().to_str().unwrap()) {
                        eprintln!("Event does not match");
                        continue;
                    }

                    let mut params = HashMap::new();
                    params.insert(
                        "github_payload".to_string(),
                        script::ScriptParameterType::String(body.clone()),
                    );
                    let job_result = job.execute(params);
                    if job_result.is_err() {
                        eprintln!("Failed to execute job: {}", job_result.unwrap());
                    } else {
                        eprintln!("Job started: {}", job_result.unwrap());
                    }
                }
                job::TriggerType::Manual(_) => {}
            }
        }
    }

    StatusCode::OK
}

pub async fn get_job_trigger_types() -> Json<serde_json::Value> {
    let job_trigger_types = job::TriggerType::get_json_schema();
    Json(job_trigger_types)
}

#[derive(Deserialize)]
pub struct JobResultsQuery {
    #[serde(rename = "job-id")]
    job_id: Option<String>,
}

pub async fn get_job_results(query: Query<JobResultsQuery>) -> Json<Vec<job::JobResult>> {
    let job_results = job::JobResult::get_all(query.job_id.clone()).unwrap();
    Json(job_results)
}

pub async fn get_job_result(Path(id): Path<String>) -> Json<job::JobResult> {
    let job_result = job::JobResult::get(id.as_str());
    Json(job_result.unwrap())
}
