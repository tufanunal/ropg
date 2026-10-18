use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::Deserialize;
use std::time::Duration;
use crate::input::InputCommand;

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct HostScreenResponse {
    pub status: Option<String>,
    #[serde(rename = "fullSyncRequested")]
    pub full_sync_requested: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HostConfig {
    pub mode: Option<String>,
    pub quality: Option<u8>,
    pub grayscale: Option<bool>,
    pub interval_ms: Option<u64>,
    pub is_running: Option<bool>,
    pub cell_size: Option<serde_json::Value>,
    pub grid_size: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct HostInputResponse {
    #[serde(rename = "fullSyncRequested")]
    pub full_sync_requested: Option<bool>,
    pub config: Option<HostConfig>,
    pub inputs: Option<Vec<InputCommand>>,
}

pub struct RelayClient {
    client: Client,
    relay_url: String,
    host_id: String,
    token: String,
}

impl RelayClient {
    pub fn new(relay_url: String, host_id: String, token: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(3000))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            relay_url,
            host_id,
            token,
        }
    }

    /// POST /api/host/:host_id/screen
    pub fn send_screen(&self, is_binary: bool, payload: &[u8], grid_preset: &str, cell_size: usize, quality: u8) -> Result<bool, String> {
        let url = format!("{}/api/host/{}/screen", self.relay_url, self.host_id);

        let mut headers = HeaderMap::new();
        if is_binary {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
            headers.insert("X-Payload-Mode", HeaderValue::from_static("binary"));
        } else {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            headers.insert("X-Payload-Mode", HeaderValue::from_static("json"));
        }

        if let Ok(val) = HeaderValue::from_str(grid_preset) {
            headers.insert("X-Grid-Size", val);
        }
        if let Ok(val) = HeaderValue::from_str(&cell_size.to_string()) {
            headers.insert("X-Cell-Size", val);
        }
        if let Ok(val) = HeaderValue::from_str(&quality.to_string()) {
            headers.insert("X-Quality", val);
        }

        if !self.token.is_empty() {
            headers.insert("X-Rogp-Token", HeaderValue::from_str(&self.token).map_err(|e| e.to_string())?);
        }

        let resp = self.client.post(&url)
            .headers(headers)
            .body(payload.to_vec())
            .send()
            .map_err(|e| format!("HTTP POST screen error: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Server returned HTTP {}", resp.status()));
        }

        let body: HostScreenResponse = resp.json().map_err(|e| format!("JSON parse error: {}", e))?;
        Ok(body.full_sync_requested.unwrap_or(false))
    }

    /// GET /api/host/:host_id/input
    pub fn poll_inputs(&self) -> Result<(bool, Option<HostConfig>, Vec<InputCommand>), String> {
        let url = format!("{}/api/host/{}/input", self.relay_url, self.host_id);

        let mut headers = HeaderMap::new();
        if !self.token.is_empty() {
            headers.insert("X-Rogp-Token", HeaderValue::from_str(&self.token).map_err(|e| e.to_string())?);
        }

        let resp = self.client.get(&url)
            .headers(headers)
            .send()
            .map_err(|e| format!("HTTP GET input error: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Server returned HTTP {}", resp.status()));
        }

        let body: HostInputResponse = resp.json().map_err(|e| format!("Input JSON parse error: {}", e))?;
        let full_sync = body.full_sync_requested.unwrap_or(false);
        let config = body.config;
        let inputs = body.inputs.unwrap_or_default();

        Ok((full_sync, config, inputs))
    }
}
