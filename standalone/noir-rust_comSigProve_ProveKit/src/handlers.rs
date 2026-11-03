use axum::{extract::State, http::StatusCode, Json};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngCore;
use std::time::Instant;
use tracing::info;

use crate::constants::*;
use crate::prover::run_prover;
use crate::signer::{poseidon_hash_payload, sign_hash, verify_signature};
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

    let mut randomness = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut randomness);


    let hash: [u8; 32] = poseidon_hash_payload(&req.data, ts, &randomness);

    let t = Instant::now();
    let sig = sign_hash(&state.signing_key, &hash)
        .map_err(|e| ise(e))?;
    info!("signing time: {:?}", t.elapsed());

    let permit = state.semaphore.clone().acquire_owned().await
        .map_err(|e| ise(e.to_string()))?;

    let (data, constraints, msg, s, rand_bytes, root, pkp, pkv) = (
        req.data.clone(),
        req.constraints.clone(),
        hash,
        sig.clone(),
        randomness,
        state.project_root.clone(),
        state.pkp_path.clone(),
        state.pkv_path.clone(),
    );

    let result = tokio::task::spawn_blocking(move || {
        let r = run_prover(
            &root, &pkp, &pkv,
            &data, &constraints, ts,
            &rand_bytes, &msg, &s,
        );
        drop(permit); 
        r
    }).await
        .map_err(|e| ise(e.to_string()))?
        .map_err(|e| ise(e))?;

    Ok(Json(ProveResponse {
        success:       true,
        proof:         B64.encode(&result.proof_np),
        vk:            B64.encode(&result.verifier_pkv),
        message:       B64.encode(&result.message),    
        signature:     B64.encode(&result.signature),  
    }))
}

pub async fn verify_handler(
    State(state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, (StatusCode, Json<ErrorResponse>)> {

    let bad = |m: &str|   (StatusCode::BAD_REQUEST,           Json(ErrorResponse { error: m.to_string() }));
    let ise = |m: String| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: m }));

    if req.proof.is_empty()     { return Err(bad("proof is required")); }
    if req.vk.is_empty()        { return Err(bad("vk is required")); }
    if req.message.is_empty()   { return Err(bad("message is required")); }
    if req.signature.is_empty() { return Err(bad("signature is required")); }

    let decode = |s: &str, name: &str|
        B64.decode(s).map_err(|e| format!("decode {}: {}", name, e));

    let message   = decode(&req.message,   "message").map_err(|e| bad(&e))?;
    let signature = decode(&req.signature, "signature").map_err(|e| bad(&e))?;

    let t = Instant::now();
    let sig_valid = verify_signature(
        &message, &signature,
        &state.issuer_pub_key_x, &state.issuer_pub_key_y,
    )
    .map_err(|e| ise(e))?;
    info!("signature verification time: {:?}", t.elapsed());

    if !sig_valid {
        info!("signature verification: FAILED");
        return Ok(Json(VerifyResponse {
            success:         true,
            valid:           false,
            signature_valid: false,
        }));
    }
    info!("signature verification: PASSED");

    let permit = state.semaphore.clone().acquire_owned().await
        .map_err(|e| ise(e.to_string()))?;

    let (proof, vk, root) = (
        req.proof.clone(),
        req.vk.clone(),
        state.project_root.clone(),
    );

    let valid = tokio::task::spawn_blocking(move || {
        let r = run_verifier(&root, &proof, &vk, &message);
        drop(permit);
        r
    }).await
        .map_err(|e| ise(e.to_string()))?
        .map_err(|e| ise(e))?;

    Ok(Json(VerifyResponse {
        success:         true,
        valid,
        signature_valid: true,
    }))
}
