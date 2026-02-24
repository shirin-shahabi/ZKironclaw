use std::time::Instant;

use crate::zkproxy::audit::ZkAuditLog;
use crate::zkproxy::config::ZkProxyConfig;
use crate::zkproxy::feature::FeatureExtractor;
use crate::zkproxy::tee::{NoopTee, TeeBackend};
use crate::zkproxy::types::{GuardDecision, ProofResult, TimingBreakdown};
use crate::zkproxy::worker::PersistentWorker;

pub struct ZkProxy {
    worker: PersistentWorker,
    extractor: FeatureExtractor,
    config: ZkProxyConfig,
    audit: ZkAuditLog,
    tee: Box<dyn TeeBackend>,
}

impl ZkProxy {
    pub async fn new(config: ZkProxyConfig) -> Result<Self, String> {
        let extractor = FeatureExtractor::from_config_file(&config.config_path)?;

        let worker =
            PersistentWorker::new(&config.python_bin, &config.worker_script).await?;

        let health = worker.health().await?;
        tracing::info!(
            "ZkProxy worker started: {}",
            serde_json::to_string(&health).unwrap_or_default()
        );

        let audit_path = config.model_path.with_extension("audit.jsonl");
        let audit = ZkAuditLog::new(audit_path, true);

        let tee: Box<dyn TeeBackend> = Box::new(NoopTee);

        Ok(Self {
            worker,
            extractor,
            config,
            audit,
            tee,
        })
    }

    pub async fn guard_check(&self, content: &str, user_id: &str) -> Result<GuardDecision, String> {
        let t_start = Instant::now();

        let t_feat = Instant::now();
        let features = self.extractor.extract(content);
        let feat_ms = t_feat.elapsed().as_secs_f64() * 1000.0;

        let params = serde_json::json!({
            "model_path": self.config.model_path.to_string_lossy(),
            "features": features,
        });

        let result_value = self.worker.call("guard_check", params).await?;
        let proof_result: ProofResult = serde_json::from_value(result_value)
            .map_err(|e| format!("Failed to parse proof result: {e}"))?;

        let timings = &proof_result.timings;
        let timing = TimingBreakdown {
            feature_extraction_ms: feat_ms,
            witness_ms: timings.get("witness_ms").copied().unwrap_or(0.0),
            prove_ms: timings.get("prove_ms").copied().unwrap_or(0.0),
            verify_ms: timings.get("verify_ms").copied().unwrap_or(0.0),
            total_ms: t_start.elapsed().as_secs_f64() * 1000.0,
        };

        let allowed = proof_result.score < self.config.threshold;

        let tee_attestation = if self.config.tee_enabled {
            let hash_bytes = hex::decode(&proof_result.proof_hash).unwrap_or_default();
            Some(self.tee.attest(&hash_bytes))
        } else {
            None
        };

        let decision = GuardDecision {
            allowed,
            score: proof_result.score,
            proof_hash: proof_result.proof_hash.clone(),
            proof_verified: proof_result.verified,
            timing: timing.clone(),
            tee_attestation: tee_attestation.clone(),
        };

        let entry = ZkAuditLog::create_entry(
            user_id,
            allowed,
            proof_result.score,
            &proof_result.proof_hash,
            proof_result.verified,
            tee_attestation,
            features,
            timing,
            self.extractor.model_hash(),
        );
        if let Err(e) = self.audit.log(&entry) {
            tracing::warn!("Failed to write ZK audit log: {e}");
        }

        Ok(decision)
    }

    pub async fn compile_guard(&self, model_path: &str) -> Result<(), String> {
        let params = serde_json::json!({ "model_path": model_path });
        let result = self.worker.call("compile", params).await?;
        let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        if !success {
            return Err(format!("Compilation failed: {result}"));
        }
        Ok(())
    }

    pub fn extractor(&self) -> &FeatureExtractor {
        &self.extractor
    }

    pub fn config(&self) -> &ZkProxyConfig {
        &self.config
    }
}
