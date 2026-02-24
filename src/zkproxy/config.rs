use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ZkProxyConfig {
    pub enabled: bool,
    pub model_path: PathBuf,
    pub config_path: PathBuf,
    pub python_bin: String,
    pub worker_script: PathBuf,
    pub threshold: f64,
    pub tee_enabled: bool,
}

impl Default for ZkProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model_path: PathBuf::from("zkproxy/guard_model.onnx"),
            config_path: PathBuf::from("zkproxy/guard_config.json"),
            python_bin: "python3".to_string(),
            worker_script: PathBuf::from("zkproxy/zkproxy_worker.py"),
            threshold: 0.5,
            tee_enabled: false,
        }
    }
}

impl ZkProxyConfig {
    pub fn from_env() -> Self {
        let enabled = std::env::var("ZKPROXY_ENABLED")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        Self {
            enabled,
            model_path: std::env::var("ZKPROXY_MODEL_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("zkproxy/guard_model.onnx")),
            config_path: std::env::var("ZKPROXY_CONFIG_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("zkproxy/guard_config.json")),
            python_bin: std::env::var("ZKPROXY_PYTHON_BIN")
                .unwrap_or_else(|_| "python3".to_string()),
            worker_script: std::env::var("ZKPROXY_WORKER_SCRIPT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("zkproxy/zkproxy_worker.py")),
            threshold: std::env::var("ZKPROXY_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.5),
            tee_enabled: std::env::var("ZKPROXY_TEE_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
        }
    }
}
