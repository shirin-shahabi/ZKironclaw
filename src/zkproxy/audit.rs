use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use chrono::Utc;
use serde::Serialize;

use crate::zkproxy::types::{AttestationReport, TimingBreakdown};

#[derive(Debug, Serialize)]
pub struct ZkAuditEntry {
    pub timestamp: String,
    pub request_id: String,
    pub user_id: String,
    pub decision: bool,
    pub score: f64,
    pub proof_hash: String,
    pub proof_verified: bool,
    pub tee_attestation: Option<AttestationReport>,
    pub feature_vector: Vec<f32>,
    pub timing: TimingBreakdown,
    pub guard_model_hash: String,
}

pub struct ZkAuditLog {
    path: PathBuf,
    enabled: bool,
}

impl ZkAuditLog {
    pub fn new(path: PathBuf, enabled: bool) -> Self {
        Self { path, enabled }
    }

    pub fn log(&self, entry: &ZkAuditEntry) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create audit dir: {e}"))?;
        }

        let line =
            serde_json::to_string(entry).map_err(|e| format!("Failed to serialize entry: {e}"))?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("Failed to open audit log: {e}"))?;

        writeln!(file, "{line}").map_err(|e| format!("Failed to write audit log: {e}"))?;
        Ok(())
    }

    pub fn create_entry(
        user_id: &str,
        decision: bool,
        score: f64,
        proof_hash: &str,
        proof_verified: bool,
        tee_attestation: Option<AttestationReport>,
        feature_vector: Vec<f32>,
        timing: TimingBreakdown,
        guard_model_hash: &str,
    ) -> ZkAuditEntry {
        ZkAuditEntry {
            timestamp: Utc::now().to_rfc3339(),
            request_id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            decision,
            score,
            proof_hash: proof_hash.to_string(),
            proof_verified,
            tee_attestation,
            feature_vector,
            timing,
            guard_model_hash: guard_model_hash.to_string(),
        }
    }
}
