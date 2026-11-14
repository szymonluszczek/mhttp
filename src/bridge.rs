//! cxx bridge to the C++ engine, its coroutines as Rust futures
//!
//! the only module with `unsafe`

use std::cell::UnsafeCell;
use std::future::Future;
use std::pin::Pin;

use cxx::UniquePtr;

use crate::{Error, Result};

#[cxx::bridge]
pub(crate) mod ffi {
    struct RustResponse {
        pub code: u32,
        pub status: String,
        pub headers: String,
        pub body: Vec<u8>,
    }

    extern "Rust" {
        type RustStatusWithVoid;
        fn produce_status_void_ok() -> Box<RustStatusWithVoid>;
        fn produce_status_void_err(status: i32) -> Box<RustStatusWithVoid>;

        type RustStatusWithResponse;
        fn produce_status_response_ok(response: RustResponse) -> Box<RustStatusWithResponse>;
        fn produce_status_response_err(status: i32) -> Box<RustStatusWithResponse>;

        fn decompress_certificate(algorithm: u16, input: &[u8], output: &mut [u8]) -> bool;
    }

    #[rustfmt::skip]
    unsafe extern "C++" {
        include!("mhttp/native/proxy.h");
        type http_proxy;

        fn new_proxy(host: &str, port: u16) -> SharedPtr<http_proxy>;
        fn new_proxy_auth(host: &str, port: u16, auth: &str) -> SharedPtr<http_proxy>;

        include!("mhttp/native/client.h");
        type RustFutureResponse = crate::bridge::RustFutureResponse;
        type RustFutureVoid = crate::bridge::RustFutureVoid;
        type http_client;

        fn new_client(host: &str, port: u16, ssl: bool, timeout: u32, version: u8) -> UniquePtr<http_client>;
        fn set_custom_hostname(self: Pin<&mut http_client>, hostname: &str);
        fn is_open(self: &http_client) -> bool;
        fn connect_direct(self: Pin<&mut http_client>) -> RustFutureVoid;
        fn connect_proxied(self: Pin<&mut http_client>, proxy: &SharedPtr<http_proxy>) -> RustFutureVoid;
        fn send_http1(self: Pin<&mut http_client>, method: &str, path: &str, headers: &str, body: &str) -> RustFutureResponse;
        fn send_http2(self: Pin<&mut http_client>, headers: &str, body: &str, depends_on: i32, weight: i32, exclusive: i32) -> RustFutureResponse;
        fn translate_error_code(status: i32) -> &'static str;

        include!("mhttp/native/ssl_preset.h");
        type ssl_preset;

        fn new_ssl_preset() -> UniquePtr<ssl_preset>;
        fn share_ssl_preset(preset: UniquePtr<ssl_preset>) -> SharedPtr<ssl_preset>;
        fn set_ssl_preset(self: Pin<&mut http_client>, preset: &SharedPtr<ssl_preset>);

        fn set_permute_extensions(self: Pin<&mut ssl_preset>, toggle: bool);
        fn set_enable_grease(self: Pin<&mut ssl_preset>, toggle: bool);
        fn set_enable_sigalg_grease(self: Pin<&mut ssl_preset>, toggle: bool);
        fn set_enable_padding(self: Pin<&mut ssl_preset>, toggle: bool);
        fn set_enable_ech_grease(self: Pin<&mut ssl_preset>, toggle: bool);
        fn set_enable_session_ticket(self: Pin<&mut ssl_preset>, toggle: bool);
        fn set_enable_ocsp_stapling(self: Pin<&mut ssl_preset>, toggle: bool);
        fn set_enable_signed_cert_timestamps(self: Pin<&mut ssl_preset>, toggle: bool);
        fn set_enable_new_alps_codepoint(self: Pin<&mut ssl_preset>, toggle: bool);
        fn set_trust_anchors(self: Pin<&mut ssl_preset>, ids: Vec<u8>);

        fn set_alpn_protos(self: Pin<&mut ssl_preset>, data: Vec<u8>);
        fn set_application_settings(self: Pin<&mut ssl_preset>, data: Vec<u8>);

        fn add_cert_compression_alg(self: Pin<&mut ssl_preset>, value: u16);
        fn add_cipher_suite(self: Pin<&mut ssl_preset>, value: u16);
        fn add_verify_algorithm_pref(self: Pin<&mut ssl_preset>, value: u16);
        fn add_group_id(self: Pin<&mut ssl_preset>, value: u16);
        fn add_supported_version(self: Pin<&mut ssl_preset>, value: u16);

        include!("mhttp/native/http2_preset.h");
        type http2_preset;

        fn new_http2_preset() -> UniquePtr<http2_preset>;
        fn share_http2_preset(preset: UniquePtr<http2_preset>) -> SharedPtr<http2_preset>;
        fn set_http2_preset(self: Pin<&mut http_client>, preset: &SharedPtr<http2_preset>);

        fn settings_add(self: Pin<&mut http2_preset>, id: i32, val: u32);
        fn set_window_update(self: Pin<&mut http2_preset>, size: i32);
        fn add_hpack_rule(self: Pin<&mut http2_preset>, name: &str, mode: u8);
        fn set_hpack_huffman(self: Pin<&mut http2_preset>, mode: u8);
        fn set_preface_with_first_request(self: Pin<&mut http2_preset>, toggle: bool);
        fn add_priority_frame(self: Pin<&mut http2_preset>, stream_id: i32, depends_on: i32, weight: i32, exclusive: bool);
    }
}

// SAFETY: presets and proxies are filled in once before being shared, then only read
unsafe impl Send for ffi::ssl_preset {}
unsafe impl Sync for ffi::ssl_preset {}
unsafe impl Send for ffi::http2_preset {}
unsafe impl Sync for ffi::http2_preset {}
unsafe impl Send for ffi::http_proxy {}
unsafe impl Sync for ffi::http_proxy {}

pub struct RustStatusWithVoid(std::result::Result<(), i32>);

fn produce_status_void_ok() -> Box<RustStatusWithVoid> {
    Box::new(RustStatusWithVoid(Ok(())))
}

fn produce_status_void_err(status: i32) -> Box<RustStatusWithVoid> {
    Box::new(RustStatusWithVoid(Err(status)))
}

pub struct RustStatusWithResponse(std::result::Result<ffi::RustResponse, i32>);

fn produce_status_response_ok(response: ffi::RustResponse) -> Box<RustStatusWithResponse> {
    Box::new(RustStatusWithResponse(Ok(response)))
}

fn produce_status_response_err(status: i32) -> Box<RustStatusWithResponse> {
    Box::new(RustStatusWithResponse(Err(status)))
}

#[cxx_async::bridge]
unsafe impl Future for RustFutureVoid {
    type Output = Box<RustStatusWithVoid>;
}

#[cxx_async::bridge]
unsafe impl Future for RustFutureResponse {
    type Output = Box<RustStatusWithResponse>;
}

// RFC 8879 certificate compression; `output` is exactly the size the server announced
fn decompress_certificate(algorithm: u16, input: &[u8], output: &mut [u8]) -> bool {
    use std::io::Read;

    let mut decoded = Vec::with_capacity(output.len());
    let limit = output.len() as u64 + 1;
    let read = match algorithm {
        1 => flate2::read::ZlibDecoder::new(input)
            .take(limit)
            .read_to_end(&mut decoded),
        2 => brotli::Decompressor::new(input, 4096)
            .take(limit)
            .read_to_end(&mut decoded),
        3 => zstd::stream::read::Decoder::new(input)
            .and_then(|decoder| decoder.take(limit).read_to_end(&mut decoded)),
        _ => return false,
    };

    if read.is_err() || decoded.len() != output.len() {
        return false;
    }
    output.copy_from_slice(&decoded);
    true
}

pub(crate) fn native_error(code: i32) -> Error {
    Error::Native {
        code,
        message: ffi::translate_error_code(code),
    }
}

// the C++ client runs its I/O on its own libuv thread and guards what it shares with
// callers (pending requests, readiness events) with its own mutex and events, so its
// methods may be called from several tasks at once (HTTP/2 multiplexing)
pub(crate) struct Client(UnsafeCell<UniquePtr<ffi::http_client>>);

// SAFETY: see above; Rust never touches the client's memory itself
unsafe impl Send for Client {}
unsafe impl Sync for Client {}

impl Client {
    pub(crate) fn new(host: &str, port: u16, tls: bool, timeout_ms: u32, version: u8) -> Self {
        Self(UnsafeCell::new(ffi::new_client(
            host, port, tls, timeout_ms, version,
        )))
    }

    pub(crate) fn is_open(&self) -> bool {
        // SAFETY: shared reference for a const method, see `pin`
        unsafe { (*self.0.get()).is_open() }
    }

    #[allow(clippy::mut_from_ref)]
    pub(crate) fn pin(&self) -> Pin<&mut ffi::http_client> {
        // SAFETY: `http_client` is opaque and zero-sized on the Rust side, so simultaneous
        // handles never alias Rust-visible memory; C++ synchronizes concurrent calls
        unsafe { (*self.0.get()).pin_mut() }
    }

    pub(crate) async fn connect(
        &self,
        proxy: Option<&cxx::SharedPtr<ffi::http_proxy>>,
    ) -> Result<()> {
        let future = match proxy {
            Some(proxy) => self.pin().connect_proxied(proxy),
            None => self.pin().connect_direct(),
        };
        future.await?.0.map_err(native_error)
    }

    // the C++ coroutines may read the string arguments until they finish, so each call is
    // awaited here while the borrows are alive

    pub(crate) async fn send_http1(
        &self,
        method: &str,
        path: &str,
        headers: &str,
        body: &str,
    ) -> Result<ffi::RustResponse> {
        let future = self.pin().send_http1(method, path, headers, body);
        future.await?.0.map_err(native_error)
    }

    pub(crate) async fn send_http2(
        &self,
        headers: &str,
        body: &str,
        (depends_on, weight, exclusive): (i32, i32, i32),
    ) -> Result<ffi::RustResponse> {
        let future = self
            .pin()
            .send_http2(headers, body, depends_on, weight, exclusive);
        future.await?.0.map_err(native_error)
    }
}
