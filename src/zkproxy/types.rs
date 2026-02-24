use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingBreakdown {
    pub feature_extraction_ms: f64,
    pub witness_ms: f64,
    pub prove_ms: f64,
    pub verify_ms: f64,
    pub total_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationReport {
    pub proof_hash: String,
    pub timestamp: String,
    pub backend: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardDecision {
    pub allowed: bool,
    pub score: f64,
    pub proof_hash: String,
    pub proof_verified: bool,
    pub timing: TimingBreakdown,
    pub tee_attestation: Option<AttestationReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofResult {
    pub success: bool,
    pub score: f64,
    pub proof_hash: String,
    pub verified: bool,
    pub timings: HashMap<String, f64>,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: &str, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    pub input_features: usize,
    pub features: Vec<FeatureSpec>,
    pub threshold: f64,
    pub model_name: String,
    #[serde(default)]
    pub model_hash_sha256: String,
    #[serde(default)]
    pub onnx_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSpec {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub index: usize,
    #[serde(default)]
    pub patterns: Vec<String>,
    #[serde(default)]
    pub strings: Vec<String>,
}
