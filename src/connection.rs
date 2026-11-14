use std::str::FromStr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use cookie::Cookie;
use cookie_store::CookieStore;
use url::Url;

use crate::bridge::{ffi, Client};
use crate::headers::{self, Target};
use crate::preset::{Http2Preset, Profile, TlsPreset};
use crate::request::{CookiePlacement, Priority, Pseudo, Request};
use crate::{Error, LogEntry, Proxy, Response, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpVersion {
    Http1 = 0,
    Http2 = 1,
}

type Logger = Box<dyn Fn(LogEntry) + Send + Sync>;

/// starts as Chrome; `tls`, `http2`, `pseudo_order` and `priority` override parts of it
pub struct ConnectionBuilder {
    url: Url,
    version: HttpVersion,
    timeout: Duration,
    proxy: Option<Proxy>,
    tls: Option<TlsPreset>,
    http2: Option<Http2Preset>,
    pseudo_order: Option<[Pseudo; 4]>,
    priority: Option<Priority>,
    connect_to: Option<String>,
    jar: Option<Arc<Mutex<CookieStore>>>,
}

impl ConnectionBuilder {
    pub fn version(mut self, version: HttpVersion) -> Self {
        self.version = version;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn proxy(mut self, proxy: Proxy) -> Self {
        self.proxy = Some(proxy);
        self
    }

    pub fn profile(mut self, profile: &Profile) -> Self {
        self.tls = Some(profile.tls.clone());
        self.http2 = Some(profile.http2.clone());
        self.pseudo_order = profile.pseudo_order;
        self.priority = profile.priority;
        self
    }

    pub fn tls(mut self, preset: TlsPreset) -> Self {
        self.tls = Some(preset);
        self
    }

    pub fn http2(mut self, preset: Http2Preset) -> Self {
        self.http2 = Some(preset);
        self
    }

    pub fn pseudo_order(mut self, order: [Pseudo; 4]) -> Self {
        self.pseudo_order = Some(order);
        self
    }

    /// for requests that don't set their own
    pub fn priority(mut self, priority: Option<Priority>) -> Self {
        self.priority = priority;
        self
    }

    /// TCP to this host instead (e.g. one edge node); SNI and headers keep the URL's host
    pub fn connect_to(mut self, host: &str) -> Self {
        self.connect_to = Some(host.to_string());
        self
    }

    pub fn cookie_jar(mut self, jar: Arc<Mutex<CookieStore>>) -> Self {
        self.jar = Some(jar);
        self
    }

    pub async fn connect(self) -> Result<Connection> {
        let host = self
            .url
            .host_str()
            .ok_or_else(|| Error::InvalidRequest(format!("{} has no host", self.url)))?
            .to_string();
        let port = self.url.port_or_known_default().unwrap_or(443);
        let tls = self.url.scheme() == "https";
        let timeout_ms = u32::try_from(self.timeout.as_millis()).unwrap_or(u32::MAX);

        let client = Client::new(&host, port, tls, timeout_ms, self.version as u8);
        if let Some(host) = &self.connect_to {
            client.pin().set_custom_hostname(host);
        }
        if let Some(preset) = &self.tls {
            client.pin().set_ssl_preset(&preset.ptr);
        }
        if let Some(preset) = &self.http2 {
            client.pin().set_http2_preset(&preset.ptr);
        }

        client
            .connect(self.proxy.as_ref().map(|proxy| &proxy.ptr))
            .await?;

        Ok(Connection {
            url: self.url,
            version: self.version,
            client,
            proxy: self.proxy,
            pseudo_order: self.pseudo_order,
            priority: self.priority,
            jar: self.jar.unwrap_or_default(),
            logger: None,
        })
    }
}

/// one origin; HTTP/2 requests may be sent concurrently from several tasks
pub struct Connection {
    url: Url,
    version: HttpVersion,
    client: Client,
    proxy: Option<Proxy>,
    pseudo_order: Option<[Pseudo; 4]>,
    priority: Option<Priority>,
    jar: Arc<Mutex<CookieStore>>,
    logger: Option<Logger>,
}

impl Connection {
    /// defaults: HTTP/2, Chrome, 30 s timeout
    pub fn builder(url: Url) -> ConnectionBuilder {
        let chrome = Profile::chrome();
        ConnectionBuilder {
            url,
            version: HttpVersion::Http2,
            timeout: Duration::from_secs(30),
            proxy: None,
            tls: Some(chrome.tls),
            http2: Some(chrome.http2),
            pseudo_order: chrome.pseudo_order,
            priority: chrome.priority,
            connect_to: None,
            jar: None,
        }
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn version(&self) -> HttpVersion {
        self.version
    }

    pub fn proxy(&self) -> Option<&Proxy> {
        self.proxy.as_ref()
    }

    pub fn is_open(&self) -> bool {
        self.client.is_open()
    }

    pub fn set_logger(&mut self, logger: impl Fn(LogEntry) + Send + Sync + 'static) {
        self.logger = Some(Box::new(logger));
    }

    pub async fn send(&self, request: Request) -> Result<Response> {
        let request_url = self.url.join(&request.path)?;
        let http2 = self.version == HttpVersion::Http2;

        let jar_cookies = match request.cookies {
            CookiePlacement::Off => Vec::new(),
            _ => self.cookies_at(&request_url),
        };

        let authority = match self.url.port() {
            Some(port) => format!("{}:{port}", self.url.host_str().unwrap_or_default()),
            None => self.url.host_str().unwrap_or_default().to_string(),
        };
        let target = Target {
            http2,
            authority: &authority,
            scheme: self.url.scheme(),
            pseudo_order: self.pseudo_order,
        };
        let header_list = headers::build(&request, &jar_cookies, &target)?;
        let joined = headers::join(&header_list, http2);

        let raw = if http2 {
            let priority = Priority::wire(request.priority.unwrap_or(self.priority));
            self.client
                .send_http2(&joined, &request.body, priority)
                .await?
        } else {
            self.client
                .send_http1(&request.method, &request.path, &joined, &request.body)
                .await?
        };

        let ffi::RustResponse {
            code,
            status,
            headers: raw_headers,
            body,
        } = raw;
        let raw_body = self.logger.as_ref().map(|_| body.clone());
        let response = Response::from_raw(code, status, &raw_headers, body)?;

        if request.cookies != CookiePlacement::Off {
            let mut jar = self.lock_jar();
            for value in response.header_all("set-cookie") {
                if let Ok(cookie) = Cookie::parse(value) {
                    _ = jar.insert_raw(&cookie, &request_url);
                }
            }
        }

        if let (Some(logger), Some(raw_body)) = (&self.logger, raw_body) {
            logger(LogEntry {
                proxy: self.proxy.as_ref().map(Proxy::url),
                request_scheme: self.url.scheme().to_string(),
                request_host: self.url.host_str().unwrap_or_default().to_string(),
                request_port: self.url.port_or_known_default().unwrap_or(443),
                request_method: request.method.clone(),
                request_path: request.path.clone(),
                request_headers: header_list,
                request_content: request.body.into_bytes(),
                response_status_code: response.status(),
                response_headers: response.headers().to_vec(),
                response_content: raw_body,
            });
        }

        Ok(response)
    }

    pub fn cookie_jar(&self) -> Arc<Mutex<CookieStore>> {
        self.jar.clone()
    }

    pub fn set_cookie_jar(&mut self, jar: Arc<Mutex<CookieStore>>) {
        self.jar = jar;
    }

    /// first cookie with this name, any path
    pub fn cookie(&self, name: &str) -> Option<String> {
        self.lock_jar()
            .iter_any()
            .find(|cookie| cookie.name() == name)
            .map(|cookie| cookie.value().to_string())
    }

    /// `Set-Cookie` values, as if from a response for `path`
    pub fn add_cookies<'a>(
        &self,
        path: &str,
        cookies: impl IntoIterator<Item = &'a str>,
    ) -> Result<()> {
        let url = self.url.join(path)?;
        let mut jar = self.lock_jar();
        for raw in cookies {
            _ = jar.insert_raw(&Cookie::from_str(raw)?, &url);
        }
        Ok(())
    }

    pub fn cookies_for(&self, path: &str) -> Result<Vec<(String, String)>> {
        Ok(self.cookies_at(&self.url.join(path)?))
    }

    fn cookies_at(&self, url: &Url) -> Vec<(String, String)> {
        self.lock_jar()
            .get_request_values(url)
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect()
    }

    fn lock_jar(&self) -> std::sync::MutexGuard<'_, CookieStore> {
        self.jar.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
