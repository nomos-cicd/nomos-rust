use axum::{
    extract::Path,
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::{
    credential::{self, YamlCredential},
    job,
    script::{self, models::Script},
};

pub async fn get_credentials() -> Json<Vec<credential::Credential>> {
    let credentials = credential::Credential::get_all().unwrap();
    Json(credentials)
}

pub async fn get_credential(Path(id): Path<String>) -> (StatusCode, Json<credential::Credential>) {
    let credential = credential::Credential::get(id.as_str());
    if credential.is_none() {
        return (StatusCode::NOT_FOUND, Json(credential.unwrap()));
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
        return (StatusCode::NOT_FOUND, Json(script.unwrap()));
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
        return (StatusCode::NOT_FOUND, Json(job.unwrap()));
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
    job.sync(None);
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

pub async fn get_job_trigger_types() -> Json<serde_json::Value> {
    let job_trigger_types = job::TriggerType::get_json_schema();
    Json(job_trigger_types)
}

pub async fn get_job_results() -> Json<Vec<job::JobResult>> {
    let job_results = job::JobResult::get_all().unwrap();
    Json(job_results)
}

pub async fn get_job_result(Path(id): Path<String>) -> Json<job::JobResult> {
    let job_result = job::JobResult::get(id.as_str());
    Json(job_result.unwrap())
}
