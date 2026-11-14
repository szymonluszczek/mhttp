use base64::Engine;
use cxx::SharedPtr;

use crate::bridge::ffi;

/// HTTP CONNECT, optional Basic auth
#[derive(Clone)]
pub struct Proxy {
    pub(crate) ptr: SharedPtr<ffi::http_proxy>,
    host: String,
    port: u16,
    credentials: Option<(String, String)>,
}

impl Proxy {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            ptr: ffi::new_proxy(host, port),
            host: host.to_string(),
            port,
            credentials: None,
        }
    }

    pub fn with_auth(host: &str, port: u16, username: &str, password: &str) -> Self {
        let token =
            base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
        Self {
            ptr: ffi::new_proxy_auth(host, port, &format!("Basic {token}")),
            host: host.to_string(),
            port,
            credentials: Some((username.to_string(), password.to_string())),
        }
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// `http://user:pass@host:port`, as most other clients take it
    pub fn url(&self) -> String {
        match &self.credentials {
            Some((user, pass)) => format!("http://{user}:{pass}@{}:{}", self.host, self.port),
            None => format!("http://{}:{}", self.host, self.port),
        }
    }
}

impl std::fmt::Debug for Proxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Proxy")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("auth", &self.credentials.is_some())
            .finish()
    }
}
