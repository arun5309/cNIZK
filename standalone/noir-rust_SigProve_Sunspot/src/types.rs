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
    pub success: bool,
    pub proof:   String,
    pub vk:      String,
    #[serde(rename = "publicInputs")]
    pub public_inputs: String,
}


#[derive(Deserialize)]
pub struct VerifyRequest {
    pub proof:         String,
    pub vk:            String,
    #[serde(rename = "publicInputs")]
    pub public_inputs: String,
}


#[derive(Serialize)]
pub struct VerifyResponse {
    pub success: bool,
    pub valid:   bool,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub struct ProveResult {
    pub proof:         Vec<u8>,
    pub vk:            Vec<u8>,
    pub public_inputs: Vec<u8>, 
}