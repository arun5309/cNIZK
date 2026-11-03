use axum::{extract::State, http::StatusCode, Json};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngCore;
use std::time::Instant;
use tracing::info;

use crate::asset::extract_asset_inputs;
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

    let (data, constraints) = extract_asset_inputs(&req.asset).map_err(bad)?;
    let loan_amount: u64 = req.loan_amount.trim().parse()
        .map_err(|_| bad(format!("loanAmount {:?} is not a valid non-negative integer", req.loan_amount)))?;

    if data.len() != DATA_COUNT {
        return Err(bad(format!("data must have exactly {} values", DATA_COUNT)));
    }
    if constraints.len() != CONSTRAINT_COUNT {
        return Err(bad(format!("constraints must have exactly {}", CONSTRAINT_COUNT)));
    }
    for (i, c) in constraints.iter().enumerate() {
        if c.operation > MAX_OP {
            return Err(bad(format!("constraints[{}].operation must be 0-{}", i, MAX_OP)));
        }
        if c.attribute_type > MAX_ATTR {
            return Err(bad(format!("constraints[{}].attribute_type must be 0-{}", i, MAX_ATTR)));
        }
    }

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as u32;

    let mut randomness = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut randomness);

    let hash: [u8; 32] = poseidon_hash_payload(&data, ts, loan_amount, &randomness);

    let t = Instant::now();
    let sig = sign_hash(&state.signing_key, &hash)
        .map_err(|e| ise(e))?;
    info!("signing time: {:?}", t.elapsed());

    let permit = state.semaphore.clone().acquire_owned().await
        .map_err(|e| ise(e.to_string()))?;

    let (data, constraints, msg, s, rand_bytes, root, pkp, pkv) = (
        data,
        constraints,
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
            &data, &constraints, ts, loan_amount,
            &rand_bytes, &msg, &s,
        );
        drop(permit);
        r
    }).await
        .map_err(|e| ise(e.to_string()))?
        .map_err(|e| ise(e))?;

    info!("proof.np size:     {} bytes ({:.2} KB)",
        result.proof_np.len(),
        result.proof_np.len() as f64 / 1024.0
    );
    info!("verifier.pkv size: {} bytes ({:.2} KB)",
        result.verifier_pkv.len(),
        result.verifier_pkv.len() as f64 / 1024.0
    );
    let pkp_size = std::fs::metadata(&state.pkp_path)
        .map(|m| m.len()).unwrap_or(0);
    info!("prover.pkp size:   {} bytes ({:.2} KB)",
        pkp_size,
        pkp_size as f64 / 1024.0
    );

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
            loan_amount:     None,
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

    let outcome = tokio::task::spawn_blocking(move || {
        let r = run_verifier(&root, &proof, &vk, &message);
        drop(permit);
        r
    }).await
        .map_err(|e| ise(e.to_string()))?
        .map_err(|e| ise(e))?;

    let accepted = outcome.filter(|o| o.message_matches);

    Ok(Json(VerifyResponse {
        success:         true,
        valid:           accepted.is_some(),
        signature_valid: true,
        loan_amount:     accepted.map(|o| o.loan_amount),
    }))
}
