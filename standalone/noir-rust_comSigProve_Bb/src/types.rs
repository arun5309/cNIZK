use serde::{Deserialize, Serialize};


#[derive(Deserialize, Clone)]
pub struct Constraint {
    pub operation: u32,
    pub value: u64,
    pub attribute_type: u32,
}


#[derive(Deserialize)]
pub struct ProveRequest {
    pub data: Vec<u64>,
    pub constraints: Vec<Constraint>,
}



#[derive(Serialize)]
pub struct ProveResponse {
    pub success: bool,
    pub proof: String,
    pub vk: String,
    #[serde(rename = "publicInputs")]
    pub public_inputs: String,
    pub message: String,
    pub signature: String,
}



#[derive(Deserialize)]
pub struct VerifyRequest {
    pub proof: String,
    pub vk: String,
    #[serde(rename = "publicInputs")]
    pub public_inputs: String,
    pub message: String,
    pub signature: String,
}


#[derive(Serialize)]
pub struct VerifyResponse {
    pub success: bool,
    pub valid: bool,
    pub signature_valid: bool,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}


pub struct ProveResult {
    pub proof: Vec<u8>,
    pub vk: Vec<u8>,
    pub public_inputs: Vec<u8>,
    pub message: Vec<u8>,
    pub signature: Vec<u8>,
}
