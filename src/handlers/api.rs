use std::collections::HashMap;

use axum::{
    extract::{Path, Query},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::{
    credential::{self, Credential},
    job,
    script::{self, models::Script},
    utils::is_signature_valid,
};

pub async fn get_credentials() -> Response {
    let credentials = credential::Credential::get_all();
    if credentials.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    Json(credentials.unwrap()).into_response()
}

pub async fn get_credential(Path(id): Path<String>) -> Response {
    let credential = credential::Credential::get(id.as_str(), None);
    if credential.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let credential = credential.unwrap();
    if credential.is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }

    Json(credential.unwrap()).into_response()
}

pub async fn create_credential(Json(credential): Json<Credential>) -> Response {
    let credential = credential::Credential::try_from(credential);
    if credential.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let credential = credential.unwrap();
    let res = credential.sync(&mut None);
    if res.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    Json(credential).into_response()
}

pub async fn delete_credential(Path(id): Path<String>) -> StatusCode {
    let credential = credential::Credential::get(id.as_str(), None);
    if credential.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    let credential = credential.unwrap();
    if credential.is_none() {
        return StatusCode::NOT_FOUND;
    }
    let res = credential.unwrap().delete();
    if res.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::NO_CONTENT
}

pub async fn get_credential_types() -> Response {
    let credential_types = credential::CredentialType::get_json_schema();
    Json(credential_types).into_response()
}

pub async fn get_scripts() -> Response {
    let scripts = Script::get_all();
    if scripts.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    Json(scripts.unwrap()).into_response()
}

pub async fn get_script(Path(id): Path<String>) -> Response {
    let script = Script::get(id.as_str());
    if script.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let script = script.unwrap();
    if script.is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }

    Json(script.unwrap()).into_response()
}

pub async fn create_script(headers: HeaderMap, body: String) -> Response {
    let content_type = headers.get("content-type");
    if content_type.is_none() {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let content_type = content_type.unwrap().to_str().unwrap();
    if content_type != "application/yaml" {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let script: Script = serde_yaml::from_str(body.as_str()).unwrap();
    let res = script.sync(None);
    if res.is_err() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    Json(script).into_response()
}

pub async fn delete_script(Path(id): Path<String>) -> StatusCode {
    let script = Script::get(id.as_str());
    if script.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    let script = script.unwrap();
    if script.is_none() {
        return StatusCode::NOT_FOUND;
    }
    let res = script.unwrap().delete();
    if res.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::NO_CONTENT
}

pub async fn get_script_parameter_types() -> Response {
    let script_parameter_types = script::ScriptParameterType::get_json_schema();
    if let Err(_) = script_parameter_types {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    Json(script_parameter_types.unwrap()).into_response()
}

pub async fn get_jobs() -> Response {
    let jobs = job::Job::get_all();
    if jobs.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    Json(jobs.unwrap()).into_response()
}

pub async fn get_job(Path(id): Path<String>) -> Response {
    let job = job::Job::get(id.as_str());
    if job.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let job = job.unwrap();
    if job.is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }

    Json(job.unwrap()).into_response()
}

pub async fn create_job(headers: HeaderMap, body: String) -> (StatusCode, String) {
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
    let res = job.sync(None);
    if let Err(err) = res {
        return (StatusCode::BAD_REQUEST, err);
    }
    (StatusCode::CREATED, job.id)
}

pub async fn delete_job(Path(id): Path<String>) -> StatusCode {
    let job = job::Job::get(id.as_str());
    if job.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    let job = job.unwrap();
    if job.is_none() {
        return StatusCode::NOT_FOUND;
    }
    let res = job.unwrap().delete();
    if res.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::NO_CONTENT
}

pub async fn execute_job(
    Path(id): Path<String>,
    /*Json(parameters): Json<HashMap<String, ScriptParameterType>>,*/
) -> (StatusCode, String) {
    let job = job::Job::get(id.as_str());
    if job.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "".to_string());
    }
    let job = job.unwrap();
    if job.is_none() {
        return (StatusCode::NOT_FOUND, "".to_string());
    }
    let result = job.unwrap().execute(Default::default());
    if result.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "".to_string());
    }

    (StatusCode::OK, result.unwrap())
}

pub async fn dry_run_job(headers: HeaderMap, body: String) -> Response {
    let content_type = headers.get("content-type");
    if content_type.is_none() {
        return (StatusCode::BAD_REQUEST, "Empty content-type".to_string()).into_response();
    }

    let content_type = content_type.unwrap().to_str().unwrap();
    if content_type != "application/yaml" {
        return (
            StatusCode::BAD_REQUEST,
            "Only application/yaml is supported".to_string(),
        )
            .into_response();
    }

    let job: job::Job = serde_yaml::from_str(body.as_str()).unwrap();
    let res = job.validate(None, Default::default());
    if let Err(err) = res {
        return (StatusCode::BAD_REQUEST, err).into_response();
    }

    StatusCode::OK.into_response()
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

                    let credential = credential::Credential::get(val.secret_credential_id.as_str(), None);
                    if credential.is_err() {
                        eprintln!("Failed to get credential: {}", val.secret_credential_id);
                        continue;
                    }
                    let credential = credential.unwrap();
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
                    if let Err(err) = is_valid {
                        eprintln!("Failed to validate signature: {}", err);
                        continue;
                    }
                    let is_valid = is_valid.unwrap();
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

pub async fn get_job_trigger_types() -> Response {
    let job_trigger_types = job::TriggerType::get_json_schema();
    if job_trigger_types.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    Json(job_trigger_types.unwrap()).into_response()
}

#[derive(Deserialize)]
pub struct JobResultsQuery {
    #[serde(rename = "job-id")]
    job_id: Option<String>,
}

pub async fn get_job_results(query: Query<JobResultsQuery>) -> Response {
    let job_results = job::JobResult::get_all(query.job_id.clone());
    if job_results.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    Json(job_results.unwrap()).into_response()
}

pub async fn get_job_result(Path(id): Path<String>) -> (StatusCode, Json<job::JobResult>) {
    let job_result = job::JobResult::get(id.as_str());
    if job_result.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(job::JobResult::create_dummy()));
    }
    let job_result = job_result.unwrap();
    if job_result.is_none() {
        return (StatusCode::NOT_FOUND, Json(job::JobResult::create_dummy()));
    }

    (StatusCode::OK, Json(job_result.unwrap()))
}
