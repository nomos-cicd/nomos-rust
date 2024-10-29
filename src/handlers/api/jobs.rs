use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::{
    credential::{Credential, CredentialType},
    job::{GithubPayload, Job, JobExecutor, TriggerType},
    script::{models::Script, ScriptParameterType},
    utils::is_signature_valid,
};

#[derive(Deserialize)]
pub struct JobsQuery {
    #[serde(rename = "script-id")]
    script_id: Option<String>,
}

pub async fn get_jobs(Query(query): Query<JobsQuery>) -> Response {
    let jobs = Job::get_all().unwrap_or_default();
    let filtered_jobs: Vec<Job> = jobs
        .into_iter()
        .filter(|job| query.script_id.as_ref().map_or(true, |id| job.script_id == *id))
        .collect();

    Json(filtered_jobs).into_response()
}

pub async fn get_job(Path(id): Path<String>) -> Response {
    match Job::get(&id) {
        Ok(Some(job)) => Json(job).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            eprintln!("Failed to get job {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_job(Json(job): Json<Job>) -> Response {
    match job.sync(None).await {
        Ok(_) => (StatusCode::CREATED, Json(job)).into_response(),
        Err(e) => {
            eprintln!("Failed to create job: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn execute_job(
    State(executor): State<Arc<JobExecutor>>,
    Path(id): Path<String>,
    Json(parameters): Json<HashMap<String, ScriptParameterType>>,
) -> Response {
    match Job::get(&id) {
        Ok(Some(job)) => match job.execute(&executor, parameters).await {
            Ok(job_result_id) => Json(job_result_id).into_response(),
            Err(e) => {
                eprintln!("Failed to execute job {}: {}", id, e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            eprintln!("Failed to get job {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_job(Path(id): Path<String>) -> Response {
    match Job::get(&id) {
        Ok(Some(job)) => match job.delete() {
            Ok(_) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => {
                eprintln!("Failed to delete job {}: {}", id, e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            eprintln!("Failed to get job {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct DryRunJobPayload {
    script: Script,
    #[serde(default)]
    parameters: HashMap<String, ScriptParameterType>,
}

pub async fn dry_run_job(Json(payload): Json<DryRunJobPayload>) -> Response {
    let job = Job::from(&payload.script);

    match job.validate(Some(&payload.script), payload.parameters).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => {
            eprintln!("Dry run failed: {}", e);
            (StatusCode::BAD_REQUEST, e).into_response()
        }
    }
}

pub async fn job_webhook_trigger(
    State(executor): State<Arc<JobExecutor>>,
    headers: HeaderMap,
    body: String,
) -> Response {
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

                                                match job.execute(&executor, params).await {
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
