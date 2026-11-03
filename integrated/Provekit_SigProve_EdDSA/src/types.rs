use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone)]
pub struct Constraint {
    pub operation:      u32,
    pub value:          u64,
    pub attribute_type: u32,
}

#[derive(Deserialize)]
pub struct ProveRequest {
    pub asset: TokenizedAsset,
    #[serde(default, rename = "loanAmount")]
    pub loan_amount: String,
}

#[derive(Deserialize, Clone)]
pub struct TokenizedAsset {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub unit: String,
    #[serde(default, rename = "creationTimestamp")]
    pub creation_timestamp: String,
    #[serde(default, rename = "issuerSignature")]
    pub issuer_signature: String,
    #[serde(default, rename = "issuerAddress")]
    pub issuer_address: String,
    #[serde(default, rename = "custodianAddress")]
    pub custodian_address: String,
    #[serde(default, rename = "ownerAddress")]
    pub owner_address: String,
    #[serde(default, rename = "type")]
    pub asset_type: String,
    #[serde(default, rename = "serialNumber")]
    pub serial_number: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default, rename = "parentId")]
    pub parent_id: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Serialize)]
pub struct ProveResponse {
    pub success: bool,
    pub proof:   String,
    pub vk:      String,
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub proof: String,
    pub vk:    String,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub success:         bool,
    pub valid:           bool,
    pub signature_valid: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loan_amount: Option<u64>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub struct ProveResult {
    pub proof_np:     Vec<u8>,
    pub verifier_pkv: Vec<u8>,
}
