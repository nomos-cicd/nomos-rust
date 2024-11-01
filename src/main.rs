mod credential;
mod docker;
mod git;
mod handlers;
mod job;
mod log;
mod script;
mod settings;
mod utils;

use axum::{routing, Router};
use axum_login::{
    login_required,
    tower_sessions::{MemoryStore, SessionManagerLayer},
    AuthManagerLayerBuilder,
};
use handlers::*;
use job::JobExecutor;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Clone)]
struct AppState {
    job_executor: Arc<JobExecutor>,
}

fn create_router() -> Router<AppState> {
    Router::new()
        .route("/api/credentials", routing::get(get_credentials))
        .route("/api/credentials/:id", routing::get(get_credential))
        .route("/api/credentials", routing::post(create_credential))
        .route("/api/credentials/:id", routing::delete(delete_credential))
        .route("/api/scripts", routing::get(get_scripts))
        .route("/api/scripts/:id", routing::get(get_script))
        .route("/api/scripts", routing::post(create_script))
        .route("/api/scripts/:id", routing::delete(delete_script))
        .route("/api/jobs", routing::get(get_jobs))
        .route("/api/jobs/:id", routing::get(get_job))
        .route("/api/jobs", routing::post(create_job))
        .route("/api/jobs/:id", routing::delete(delete_job))
        .route("/api/jobs/:id/execute", routing::post(execute_job))
        .route("/api/jobs/dry-run", routing::post(dry_run_job))
        .route("/api/job-results", routing::get(get_job_results))
        .route("/api/job-results/:id", routing::get(get_job_result))
        .route("/api/job-results/:id/stop", routing::post(stop_job))
        .route("/", routing::get(template_job_results))
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
        .route("/job-results/table", routing::get(template_job_results_table))
        .route("/job-results/:id", routing::get(template_job_result))
        .route("/job-results/:id/logs", routing::get(template_job_result_logs))
        .route(
            "/job-results/:id/:content_type",
            routing::get(template_job_result_dynamic_content),
        )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !cfg!(debug_assertions) {
        let _ = std::env::var("NOMOS_USERNAME").map_err(|_| {
            eprintln!("NOMOS_USERNAME environment variable is not set.");
            std::process::exit(1);
        });
        let _ = std::env::var("NOMOS_PASSWORD").map_err(|_| {
            eprintln!("NOMOS_PASSWORD environment variable is not set.");
            std::process::exit(1);
        });
    }

    // initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "axum_login=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;

    // Session layer.
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store);

    // Auth service.
    let backend = Backend::default();
    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

    let mut app = create_router();

    // Only add authentication layer in release mode
    if !cfg!(debug_assertions) {
        app = app.route_layer(login_required!(Backend, login_url = "/login"));
    }

    app = app
        .route("/login", routing::get(template_get_login))
        .route("/login", routing::post(template_post_login))
        .route("/public/api/webhook", routing::post(job_webhook_trigger))
        .layer(auth_layer)
        .layer(CorsLayer::permissive());

    // Apply state to the router
    let app_state = AppState {
        job_executor: Arc::new(JobExecutor::new()),
    };
    let app = app.with_state(app_state);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .map_err(|e| e.to_string())?;
    axum::serve(listener, app).await.map_err(|e| e.into())
}
