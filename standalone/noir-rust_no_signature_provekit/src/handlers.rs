use axum::{extract::State, http::StatusCode, Json};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use std::time::Instant;
use tracing::info;

use crate::constants::*;
use crate::prover::run_prover;
use crate::state::AppState;
use crate::types::{ErrorResponse, ProveRequest, ProveResponse, VerifyRequest, VerifyResponse};
use crate::verifier::run_verifier;

pub async fn prove_handler(
    State(state): State<AppState>,
    Json(req): Json<ProveRequest>,
) -> Result<Json<ProveResponse>, (StatusCode, Json<ErrorResponse>)> {

    let bad = |m: String| (StatusCode::BAD_REQUEST,           Json(ErrorResponse { error: m }));
    let ise = |m: String| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: m }));

    if req.data.len() != DATA_COUNT {
        return Err(bad(format!("data must have exactly {} values", DATA_COUNT)));
    }
    for (i, &v) in req.data.iter().enumerate() {
        if v < 0 || v > MAX_U32 {
            return Err(bad(format!("data[{}] out of u32 range", i)));
        }
    }
    if req.constraints.len() != CONSTRAINT_COUNT {
        return Err(bad(format!("constraints must have exactly {}", CONSTRAINT_COUNT)));
    }
    for (i, c) in req.constraints.iter().enumerate() {
        if c.operation < 0 || c.operation > MAX_OP {
            return Err(bad(format!("constraints[{}].operation must be 0-{}", i, MAX_OP)));
        }
        if c.attribute_type != 0 && c.attribute_type != 2 {
            return Err(bad(format!("constraints[{}].attribute_type must be 0 (integer) or 2 (boolean)", i)));
        }
        if c.value < 0 || c.value > MAX_U32 {
            return Err(bad(format!("constraints[{}].value out of u32 range", i)));
        }
    }

    let permit = state.semaphore.clone().acquire_owned().await
        .map_err(|e| ise(e.to_string()))?;

    let (data, constraints, root, pkp, pkv) = (
        req.data.clone(),
        req.constraints.clone(),
        state.project_root.clone(),
        state.pkp_path.clone(),
        state.pkv_path.clone(),
    );

    let t_total = Instant::now();

    
    let result = tokio::task::spawn_blocking(move || {
        let r = run_prover(&root, &pkp, &pkv, &data, &constraints);
        drop(permit);
        r
    }).await
        .map_err(|e| ise(e.to_string()))?
        .map_err(|e| ise(e))?;

    info!("prove_handler total wall time: {:?}", t_total.elapsed());

    Ok(Json(ProveResponse {
        success: true,
        proof:   B64.encode(&result.proof),
        pkv:     B64.encode(&result.pkv),
    }))
}

pub async fn verify_handler(
    State(state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, (StatusCode, Json<ErrorResponse>)> {

    let bad = |m: &str|  (StatusCode::BAD_REQUEST,           Json(ErrorResponse { error: m.to_string() }));
    let ise = |m: String|(StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: m }));

    if req.proof.is_empty() { return Err(bad("proof is required")); }
    if req.pkv.is_empty()   { return Err(bad("pkv is required")); }

    let permit = state.semaphore.clone().acquire_owned().await
        .map_err(|e| ise(e.to_string()))?;

    let (proof, pkv, root) = (
        req.proof.clone(),
        req.pkv.clone(),
        state.project_root.clone(),
    );

    let t_total = Instant::now();

    let valid = tokio::task::spawn_blocking(move || {
        let r = run_verifier(&root, &proof, &pkv);
        drop(permit);
        r
    }).await
        .map_err(|e| ise(e.to_string()))?
        .map_err(|e| ise(e))?;

    info!("verify_handler total wall time: {:?}", t_total.elapsed());

    Ok(Json(VerifyResponse { success: true, valid }))
}
