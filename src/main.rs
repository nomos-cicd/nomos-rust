mod credential;
mod git;
mod job;
mod log;
mod script;
mod settings;
mod utils;

use axum::{extract::Path, http::StatusCode, routing, Json, Router};
use credential::YamlCredential;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // build our application with a route
    let app = Router::new()
        .route("/credentials", routing::get(get_credentials))
        .route("/credentials", routing::post(create_credential))
        .route("/credentials/:id", routing::delete(delete_credential))
        .route("/scripts", routing::get(get_scripts))
        .route("/scripts", routing::post(create_script))
        .route("/scripts/:id", routing::delete(delete_script))
        .route("/jobs", routing::get(get_jobs))
        .route("/jobs", routing::post(create_job))
        .route("/jobs/:id", routing::delete(delete_job));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_credentials() -> Json<Vec<credential::Credential>> {
    let credentials = credential::Credential::get_all();
    Json(credentials)
}

async fn create_credential(Json(credential): Json<YamlCredential>) -> Json<credential::Credential> {
    let credential = credential::Credential::try_from(credential).unwrap();
    credential.sync(None);
    Json(credential)
}

async fn delete_credential(Path(id): Path<String>) -> StatusCode {
    let credential = credential::Credential::get(id.as_str());
    if let None = credential {
        return StatusCode::NOT_FOUND;
    }
    credential.unwrap().delete();
    StatusCode::NO_CONTENT
}

async fn get_scripts() -> Json<Vec<script::Script>> {
    let scripts = script::Script::get_all();
    Json(scripts)
}

async fn create_script(Json(script): Json<script::Script>) -> Json<script::Script> {
    let script = script::Script::try_from(script).unwrap();
    script.sync(None);
    Json(script)
}

async fn delete_script(Path(id): Path<String>) -> StatusCode {
    let script = script::Script::get(id.as_str());
    if let None = script {
        return StatusCode::NOT_FOUND;
    }
    script.unwrap().delete();
    StatusCode::NO_CONTENT
}

async fn get_jobs() -> Json<Vec<job::Job>> {
    let jobs = job::Job::get_all();
    Json(jobs)
}

async fn create_job(Json(job): Json<job::Job>) -> Json<job::Job> {
    let job = job::Job::try_from(job).unwrap();
    job.sync(None);
    Json(job)
}

async fn delete_job(Path(id): Path<String>) -> StatusCode {
    let job = job::Job::get(id.as_str());
    if let None = job {
        return StatusCode::NOT_FOUND;
    }
    job.unwrap().delete();
    StatusCode::NO_CONTENT
}
