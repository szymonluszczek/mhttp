//! values from captured browser traffic; build your own from [`TlsConfig`] and
//! [`Http2Config`]

mod http2;
mod tls;

pub use http2::{setting, HpackRule, Http2Config, Http2Preset, Huffman, Literal, PriorityFrame};
pub use tls::{cert_compression, cipher, group, sigalg, version, TlsConfig, TlsPreset};

use crate::request::{Priority, Pseudo};

#[derive(Clone)]
pub struct Profile {
    pub tls: TlsPreset,
    pub http2: Http2Preset,
    pub pseudo_order: Option<[Pseudo; 4]>,
    pub priority: Option<Priority>,
}

impl Profile {
    /// Chrome 157; needs a BoringSSL with ML-DSA sigalgs and trust_anchors (README)
    pub fn chrome() -> Self {
        Self {
            tls: TlsPreset::chrome(),
            http2: Http2Preset::chrome(),
            pseudo_order: Some(Pseudo::CHROME),
            priority: Some(Priority::CHROME),
        }
    }

    /// Safari 27 (iOS 27), RFC 7540 priorities off
    pub fn safari_ios() -> Self {
        Self {
            tls: TlsPreset::safari_ios(),
            http2: Http2Preset::safari_ios(),
            pseudo_order: Some(Pseudo::SAFARI),
            priority: None,
        }
    }
}
