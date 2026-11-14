//! HTTP/1.1 and HTTP/2 client with a configurable TLS ClientHello and HTTP/2 preface, to
//! look like a chosen browser on the wire
//!
//! async Rust API over a C++ engine (BoringSSL, nghttp2, libuv)
//!
//! ```no_run
//! # async fn run() -> mhttp::Result<()> {
//! use mhttp::{Connection, CookiePlacement, Profile, Request};
//!
//! let url = "https://tls.peet.ws".parse().unwrap();
//! let connection = Connection::builder(url)
//!     .profile(&Profile::chrome())
//!     .connect()
//!     .await?;
//!
//! let request = Request::get("/api/all")
//!     .header("accept", "application/json")
//!     .cookies(CookiePlacement::Last)
//!     .build();
//! let response = connection.send(request).await?;
//! println!("{} {}", response.status(), response.text());
//! # Ok(())
//! # }
//! ```

mod bridge;
mod connection;
mod error;
mod headers;
mod log;
pub mod preset;
mod proxy;
mod request;
mod response;

pub use connection::{Connection, ConnectionBuilder, HttpVersion};
pub use error::{Error, Result};
pub use log::LogEntry;
pub use preset::Profile;
pub use proxy::Proxy;
pub use request::{CookiePlacement, Priority, Pseudo, Request, RequestBuilder};
pub use response::Response;

pub use cookie_store;
pub use url;
