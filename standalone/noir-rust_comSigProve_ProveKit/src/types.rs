use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone)]
pub struct Constraint {
    pub operation:      i64,
    pub value:          i64,
    pub attribute_type: i64,
}

#[derive(Deserialize)]
pub struct ProveRequest {
    pub data:        Vec<i64>,
    pub constraints: Vec<Constraint>,
}


#[derive(Serialize)]
pub struct ProveResponse {
    pub success:   bool,
    pub proof:     String,     
    pub vk:        String,     
    pub message:   String,     
    pub signature: String,     
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub proof:     String,     
    pub vk:        String,     
    pub message:   String,     
    pub signature: String,     
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub success:         bool,
    pub valid:           bool,
    pub signature_valid: bool, 
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub struct ProveResult {
    pub proof_np:     Vec<u8>, 
    pub verifier_pkv: Vec<u8>, 
    pub message:      Vec<u8>, 
    pub signature:    Vec<u8>,
}