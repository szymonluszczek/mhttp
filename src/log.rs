use base64_serde::base64_serde_type;
use serde::Serialize;

base64_serde_type!(Base64Standard, base64::engine::general_purpose::STANDARD);

/// JSON with base64 bodies, close to mitmproxy's flow format
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub proxy: Option<String>,
    pub request_scheme: String,
    pub request_host: String,
    pub request_port: u16,
    pub request_method: String,
    pub request_path: String,
    /// exactly as sent, generated ones and cookies included
    pub request_headers: Vec<(String, String)>,
    #[serde(with = "Base64Standard")]
    pub request_content: Vec<u8>,
    pub response_status_code: u16,
    pub response_headers: Vec<(String, String)>,
    /// still compressed
    #[serde(with = "Base64Standard")]
    pub response_content: Vec<u8>,
}
