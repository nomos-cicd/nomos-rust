use std::collections::HashMap;

use axum::{
    extract::Path,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

use crate::{
    credential::{Credential, CredentialType},
    job::{GithubPayload, Job, TriggerType},
    script::ScriptParameterType,
    utils::is_signature_valid,
};

pub async fn get_jobs() -> Response {
    match Job::get_all() {
        Ok(jobs) => Json(jobs).into_response(),
        Err(e) => {
            eprintln!("Failed to get jobs: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_job(Path(id): Path<String>) -> Response {
    match Job::get(id.as_str()) {
        Ok(Some(job)) => Json(job).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            eprintln!("Failed to get job {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_job(headers: HeaderMap, body: String) -> Response {
    let content_type = match headers.get("content-type") {
        Some(ct) => ct.to_str().unwrap_or(""),
        None => return (StatusCode::BAD_REQUEST, "Empty content-type").into_response(),
    };

    if content_type != "application/yaml" {
        return (StatusCode::BAD_REQUEST, "Only application/yaml is supported").into_response();
    }

    match serde_yaml::from_str::<Job>(&body) {
        Ok(job) => match job.sync(None) {
            Ok(_) => (StatusCode::CREATED, job.id).into_response(),
            Err(e) => {
                eprintln!("Failed to sync job: {}", e);
                (StatusCode::BAD_REQUEST, e).into_response()
            }
        },
        Err(e) => {
            eprintln!("Failed to parse job YAML: {}", e);
            (StatusCode::BAD_REQUEST, e.to_string()).into_response()
        }
    }
}

pub async fn delete_job(Path(id): Path<String>) -> Response {
    match Job::get(id.as_str()) {
        Ok(Some(job)) => match job.delete() {
            Ok(_) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => {
                eprintln!("Failed to delete job {}: {}", id, e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            eprintln!("Failed to get job for deletion {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn execute_job(Path(id): Path<String>) -> Response {
    match Job::get(id.as_str()) {
        Ok(Some(job)) => match job.execute(Default::default()) {
            Ok(result) => (StatusCode::OK, result).into_response(),
            Err(e) => {
                eprintln!("Failed to execute job {}: {}", id, e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            eprintln!("Failed to get job for execution {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn dry_run_job(headers: HeaderMap, body: String) -> Response {
    let content_type = match headers.get("content-type") {
        Some(ct) => ct.to_str().unwrap_or(""),
        None => return (StatusCode::BAD_REQUEST, "Empty content-type").into_response(),
    };

    if content_type != "application/yaml" {
        return (StatusCode::BAD_REQUEST, "Only application/yaml is supported").into_response();
    }

    match serde_yaml::from_str::<Job>(&body) {
        Ok(job) => match job.validate(None, Default::default()) {
            Ok(_) => StatusCode::OK.into_response(),
            Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
        },
        Err(e) => {
            eprintln!("Failed to parse job YAML: {}", e);
            (StatusCode::BAD_REQUEST, e.to_string()).into_response()
        }
    }
}

pub async fn job_webhook_trigger(headers: HeaderMap, body: String) -> Response {
    match Job::get_all() {
        Ok(jobs) => {
            for job in jobs {
                for trigger in job.triggers.iter() {
                    match trigger {
                        TriggerType::Github(val) => {
                            let signature = headers.get("x-hub-signature-256");
                            let github_event = headers.get("x-github-event");

                            if signature.is_none() || github_event.is_none() {
                                eprintln!("Signature or Event not found in headers");
                                continue;
                            }

                            let payload = match serde_json::from_str::<GithubPayload>(&body) {
                                Ok(p) => p,
                                Err(e) => {
                                    eprintln!("Failed to parse GitHub payload: {}", e);
                                    continue;
                                }
                            };

                            match Credential::get(val.secret_credential_id.as_str(), None) {
                                Ok(Some(credential)) => {
                                    let text_credential = match credential.value {
                                        CredentialType::Text(val) => Some(val),
                                        _ => {
                                            eprintln!("Credential is not Text: {}", val.secret_credential_id);
                                            None
                                        }
                                    };

                                    if let Some(text_credential) = text_credential {
                                        match is_signature_valid(
                                            &body,
                                            signature.unwrap().to_str().unwrap(),
                                            &text_credential.value,
                                        ) {
                                            Ok(is_valid) => {
                                                if !is_valid {
                                                    eprintln!("Invalid signature");
                                                    continue;
                                                }

                                                if payload.repository.full_name != val.url {
                                                    eprintln!("Repository does not match");
                                                    continue;
                                                }

                                                if !val
                                                    .events
                                                    .iter()
                                                    .any(|x| x == github_event.unwrap().to_str().unwrap())
                                                {
                                                    eprintln!("Event does not match");
                                                    continue;
                                                }

                                                let mut params = HashMap::new();
                                                params.insert(
                                                    "github_payload".to_string(),
                                                    ScriptParameterType::String(body.clone()),
                                                );

                                                match job.execute(params) {
                                                    Ok(result) => eprintln!("Job started: {}", result),
                                                    Err(e) => eprintln!("Failed to execute job: {}", e),
                                                }
                                            }
                                            Err(e) => eprintln!("Failed to validate signature: {}", e),
                                        }
                                    }
                                }
                                Ok(None) => eprintln!("Credential not found: {}", val.secret_credential_id),
                                Err(e) => eprintln!("Failed to get credential: {}", e),
                            }
                        }
                        TriggerType::Manual(_) => {}
                    }
                }
            }
            StatusCode::OK.into_response()
        }
        Err(e) => {
            eprintln!("Failed to get jobs for webhook trigger: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_job_trigger_types() -> Response {
    match TriggerType::get_json_schema() {
        Ok(types) => Json(types).into_response(),
        Err(e) => {
            eprintln!("Failed to get job trigger types: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
