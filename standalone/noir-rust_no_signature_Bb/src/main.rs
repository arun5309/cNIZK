mod constants;
mod handlers;
mod prover;
mod state;
mod types;
mod utils;
mod verifier;

use axum::{routing::post, Router};
use std::sync::Arc;
use std::process::Command;
use tokio::sync::Semaphore;
use tracing::{error, info};

use handlers::{prove_handler, verify_handler};
use prover::precompute_vk;
use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let _ = dotenvy::dotenv();

    let project_root = std::env::current_dir().expect("cannot get current dir");
    info!("project root: {}", project_root.display());

    let proofsystem_dir = project_root.join("proofsystem");
    let json_path = proofsystem_dir.join("target").join("proofsystem.json");
    let circuit_src = proofsystem_dir.join("src").join("main.nr");

    let circuit_stale = match (std::fs::metadata(&json_path), std::fs::metadata(&circuit_src)) {
        (Ok(json_m), Ok(src_m)) => match (src_m.modified(), json_m.modified()) {
            (Ok(src_t), Ok(json_t)) => src_t > json_t,
            _ => false,
        },
        (Err(_), _) => true,
        _ => false,
    };

    if circuit_stale {
        info!("compiling noir circuit...");
        let status = Command::new("nargo")
            .arg("compile")
            .current_dir(&proofsystem_dir)
            .status()
            .expect("failed to run nargo compile");
        if !status.success() {
            error!("nargo compile failed");
            std::process::exit(1);
        }
        info!("circuit compiled");
    } else {
        info!("circuit already compiled, skipping compile");
    }

    let vk_path = match precompute_vk(&project_root) {
        Ok(path) => path,
        Err(e) => {
            error!("failed to precompute VK: {}", e);
            std::process::exit(1);
        }
    };

    let workers = std::env::var("WORKER_COUNT").ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(num_cpus::get);
    info!("workers: {}", workers);

    let state = AppState {
        semaphore:    Arc::new(Semaphore::new(workers)),
        project_root,
        vk_path,
    };

    let app = Router::new()
        .route("/proof/prove",  post(prove_handler))
        .route("/proof/verify", post(verify_handler))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "4000".to_string());
    info!("server running on port {}", port);

    axum::serve(
        tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
            .await.unwrap(),
        app,
    ).await.unwrap();
}
