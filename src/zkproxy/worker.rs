use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use crate::zkproxy::types::{JsonRpcRequest, JsonRpcResponse};

pub struct PersistentWorker {
    child: Mutex<Option<Child>>,
    stdin: Mutex<Option<tokio::process::ChildStdin>>,
    reader: Mutex<Option<BufReader<tokio::process::ChildStdout>>>,
    python_bin: String,
    worker_script: String,
    request_id: AtomicU64,
}

impl PersistentWorker {
    pub async fn new(python_bin: &str, worker_script: &Path) -> Result<Self, String> {
        let worker = Self {
            child: Mutex::new(None),
            stdin: Mutex::new(None),
            reader: Mutex::new(None),
            python_bin: python_bin.to_string(),
            worker_script: worker_script.to_string_lossy().to_string(),
            request_id: AtomicU64::new(1),
        };
        worker.spawn().await?;
        worker.wait_for_startup().await?;
        Ok(worker)
    }

    async fn spawn(&self) -> Result<(), String> {
        let mut cmd = Command::new(&self.python_bin);
        cmd.arg(&self.worker_script)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn worker: {e}"))?;

        let stdin = child.stdin.take().ok_or("Failed to get worker stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to get worker stdout")?;

        *self.child.lock().await = Some(child);
        *self.stdin.lock().await = Some(stdin);
        *self.reader.lock().await = Some(BufReader::new(stdout));

        Ok(())
    }

    async fn wait_for_startup(&self) -> Result<(), String> {
        let mut reader_guard = self.reader.lock().await;
        let reader = reader_guard.as_mut().ok_or("No reader available")?;

        let mut line = String::new();
        tokio::time::timeout(std::time::Duration::from_secs(30), reader.read_line(&mut line))
            .await
            .map_err(|_| "Worker startup timeout".to_string())?
            .map_err(|e| format!("Failed to read startup message: {e}"))?;

        let msg: serde_json::Value =
            serde_json::from_str(&line).map_err(|e| format!("Invalid startup message: {e}"))?;

        if msg.get("params").and_then(|p| p.get("status")).and_then(|s| s.as_str()) != Some("ready")
        {
            return Err(format!("Unexpected startup message: {line}"));
        }

        Ok(())
    }

    pub async fn call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = self.request_id.fetch_add(1, Ordering::Relaxed);
        let request = JsonRpcRequest::new(id, method, params);
        let request_line =
            serde_json::to_string(&request).map_err(|e| format!("Serialize error: {e}"))?;

        let mut stdin_guard = self.stdin.lock().await;
        let stdin = stdin_guard.as_mut().ok_or("Worker stdin unavailable")?;
        stdin
            .write_all(format!("{request_line}\n").as_bytes())
            .await
            .map_err(|e| format!("Failed to write to worker: {e}"))?;
        stdin
            .flush()
            .await
            .map_err(|e| format!("Failed to flush worker stdin: {e}"))?;
        drop(stdin_guard);

        let mut reader_guard = self.reader.lock().await;
        let reader = reader_guard.as_mut().ok_or("Worker stdout unavailable")?;

        let mut response_line = String::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(120),
            reader.read_line(&mut response_line),
        )
        .await
        .map_err(|_| "Worker response timeout".to_string())?
        .map_err(|e| format!("Failed to read from worker: {e}"))?;

        let response: JsonRpcResponse = serde_json::from_str(&response_line)
            .map_err(|e| format!("Invalid response: {e} -- raw: {response_line}"))?;

        if let Some(err) = response.error {
            return Err(format!("Worker error {}: {}", err.code, err.message));
        }

        response.result.ok_or_else(|| "Empty result from worker".to_string())
    }

    pub async fn health(&self) -> Result<serde_json::Value, String> {
        self.call("health", serde_json::json!({})).await
    }

    pub async fn is_alive(&self) -> bool {
        let guard = self.child.lock().await;
        if let Some(child) = guard.as_ref() {
            child.id().is_some()
        } else {
            false
        }
    }

    pub async fn restart(&self) -> Result<(), String> {
        {
            let mut child_guard = self.child.lock().await;
            if let Some(mut child) = child_guard.take() {
                let _ = child.kill().await;
            }
        }
        self.spawn().await?;
        self.wait_for_startup().await
    }
}

impl Drop for PersistentWorker {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.child.try_lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.start_kill();
            }
        }
    }
}
