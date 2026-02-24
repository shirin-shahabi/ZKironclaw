use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::zkproxy::types::AttestationReport;

pub trait TeeBackend: Send + Sync {
    fn attest(&self, proof_hash: &[u8]) -> AttestationReport;
    fn verify_attestation(&self, report: &AttestationReport) -> bool;
    fn name(&self) -> &str;
}

pub struct NoopTee;

impl TeeBackend for NoopTee {
    fn attest(&self, proof_hash: &[u8]) -> AttestationReport {
        let timestamp = Utc::now().to_rfc3339();
        let mut hasher = Sha256::new();
        hasher.update(proof_hash);
        hasher.update(timestamp.as_bytes());
        hasher.update(b"noop-tee-self-signed");
        let signature = hex::encode(hasher.finalize());

        AttestationReport {
            proof_hash: hex::encode(proof_hash),
            timestamp,
            backend: "noop".to_string(),
            signature,
        }
    }

    fn verify_attestation(&self, report: &AttestationReport) -> bool {
        let proof_bytes = hex::decode(&report.proof_hash).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(&proof_bytes);
        hasher.update(report.timestamp.as_bytes());
        hasher.update(b"noop-tee-self-signed");
        let expected = hex::encode(hasher.finalize());
        expected == report.signature
    }

    fn name(&self) -> &str {
        "noop"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_tee_roundtrip() {
        let tee = NoopTee;
        let proof_hash = b"test_proof_hash_123";
        let report = tee.attest(proof_hash);
        assert!(tee.verify_attestation(&report));
        assert_eq!(report.backend, "noop");
    }

    #[test]
    fn noop_tee_rejects_tampered() {
        let tee = NoopTee;
        let report = tee.attest(b"real_hash");
        let mut tampered = report;
        tampered.proof_hash = hex::encode(b"fake_hash");
        assert!(!tee.verify_attestation(&tampered));
    }
}
