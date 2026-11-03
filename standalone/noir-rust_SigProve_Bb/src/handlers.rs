use axum::{extract::State, http::StatusCode, Json};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use std::time::Instant;
use tracing::info;

use crate::constants::*;
use crate::prover::run_prover;
use crate::signer::{poseidon_hash_inputs, sign_hash};
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
    
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as u32;

 
    let hash: [u8; 32] = poseidon_hash_inputs(&req.data, ts);

    let t   = Instant::now();
    let sig = sign_hash(&state.signing_key, &hash).map_err(|e| ise(e))?;
    info!("signing time: {:?}", t.elapsed());


    let (pkx, pky) = (state.issuer_pub_key_x, state.issuer_pub_key_y);

    let permit = state.semaphore.clone().acquire_owned().await
        .map_err(|e| ise(e.to_string()))?;


    let (data, constraints, s, x, y, root, vk) = (
        req.data.clone(),          
        req.constraints.clone(),   
        sig,                      
        pkx,                       
        pky,                      
        state.project_root.clone(),
        state.vk_path.clone(),     
    );

    let result = tokio::task::spawn_blocking(move || {
        let r = run_prover(&root, &vk, &data, &constraints, ts, &s, &x, &y);
        drop(permit); 
        r
    }).await
        .map_err(|e| ise(e.to_string()))?
        .map_err(|e| ise(e))?;

    info!("proof size:         {} bytes ({:.2} KB)", 
        result.proof.len(), result.proof.len() as f64 / 1024.0);
    info!("public_inputs size: {} bytes ({:.2} KB)", 
        result.public_inputs.len(), result.public_inputs.len() as f64 / 1024.0);
    info!("vk size:            {} bytes ({:.2} KB)", 
        result.vk.len(), result.vk.len() as f64 / 1024.0);
    
    Ok(Json(ProveResponse {
        success:       true,
        proof:         B64.encode(&result.proof),
        vk:            B64.encode(&result.vk),
        public_inputs: B64.encode(&result.public_inputs),
    }))
}

pub async fn verify_handler(
    State(state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, (StatusCode, Json<ErrorResponse>)> {

    let bad = |m: &str|  (StatusCode::BAD_REQUEST,           Json(ErrorResponse { error: m.to_string() }));
    let ise = |m: String|(StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: m }));


    if req.proof.is_empty()         { return Err(bad("proof is required")); }
    if req.vk.is_empty()            { return Err(bad("vk is required")); }
    if req.public_inputs.is_empty() { return Err(bad("publicInputs is required")); }

    let permit = state.semaphore.clone().acquire_owned().await
        .map_err(|e| ise(e.to_string()))?;

    let (proof, vk, pub_in, root, issuer_x, issuer_y) = (
        req.proof.clone(),
        req.vk.clone(),
        req.public_inputs.clone(),
        state.project_root.clone(),
        state.issuer_pub_key_x,
        state.issuer_pub_key_y,
    );

    let valid = tokio::task::spawn_blocking(move || {
        let r = run_verifier(&root, &proof, &vk, &pub_in, &issuer_x, &issuer_y);
        drop(permit);
        r
    }).await
        .map_err(|e| ise(e.to_string()))?
        .map_err(|e| ise(e))?;

    Ok(Json(VerifyResponse { success: true, valid }))
}
