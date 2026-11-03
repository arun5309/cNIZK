mod constants;
mod handlers;
mod prover;
mod signer;
mod state;
mod types;
mod utils;
mod verifier;

use axum::{extract::DefaultBodyLimit, routing::post, Router};
use std::sync::Arc;
use std::process::Command;
use tokio::sync::Semaphore;
use tracing::{error, info};

use handlers::{prove_handler, verify_handler};
use prover::precompute_keys;
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

    let signing_key: [u8; 32] = {
        let hex_str = std::env::var("SIGNING_KEY").expect("SIGNING_KEY not set");
        let bytes   = hex::decode(&hex_str).expect("SIGNING_KEY not valid hex");
        assert!(bytes.len() == 32, "SIGNING_KEY must be exactly 32 bytes");
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        arr
    };
    info!("signer initialized");

    let (issuer_pub_key_x, issuer_pub_key_y) = signer::get_pub_key_xy(&signing_key);

    let proofsystem_dir = project_root.join("proofsystem");
    let target_dir = proofsystem_dir.join("target");
    let json_path = target_dir.join("proofsystem.json");

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

        let status = Command::new("provekit-cli")
            .args([
                "compile",
                "--program-dir",
                proofsystem_dir.to_str().unwrap(),
                "--target-dir",
                target_dir.to_str().unwrap(),
            ])
            .status()
            .expect("failed to run provekit-cli compile");

        if !status.success() {
            error!("provekit-cli compile failed");
            std::process::exit(1);
        }

        info!("circuit compiled");
    } else {
        info!("circuit already compiled, skipping compile");
    }
    
    let keys_dir = project_root.join("keys");
    std::fs::create_dir_all(&keys_dir).expect("failed to create keys/ dir");

    let (pkp_path, pkv_path) = match precompute_keys(&project_root) {
        Ok(paths) => paths,
        Err(e) => {
            error!("failed to prepare ProveKit keys: {}", e);
            std::process::exit(1);
        }
    };

    let workers = std::env::var("WORKER_COUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(num_cpus::get);
    info!("workers: {}", workers);

    let state = AppState {
        signing_key,
        issuer_pub_key_x,
        issuer_pub_key_y,
        semaphore:    Arc::new(Semaphore::new(workers)),    
        project_root,
        pkp_path,
        pkv_path,
    };

    let app = Router::new()
        .route("/proof/prove",  post(prove_handler))
        .route("/proof/verify", post(verify_handler))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "4000".to_string());
    info!("server running on port {}", port);

    axum::serve(
        tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
            .await
            .unwrap(),
        app,
    )
    .await
    .unwrap();
}
