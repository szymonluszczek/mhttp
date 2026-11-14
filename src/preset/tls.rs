use cxx::SharedPtr;

use crate::bridge::ffi;

pub mod cipher {
    pub const TLS_AES_128_GCM_SHA256: u16 = 0x1301;
    pub const TLS_AES_256_GCM_SHA384: u16 = 0x1302;
    pub const TLS_CHACHA20_POLY1305_SHA256: u16 = 0x1303;
    pub const ECDHE_ECDSA_AES128_GCM_SHA256: u16 = 0xc02b;
    pub const ECDHE_RSA_AES128_GCM_SHA256: u16 = 0xc02f;
    pub const ECDHE_ECDSA_AES256_GCM_SHA384: u16 = 0xc02c;
    pub const ECDHE_RSA_AES256_GCM_SHA384: u16 = 0xc030;
    pub const ECDHE_ECDSA_CHACHA20_POLY1305: u16 = 0xcca9;
    pub const ECDHE_RSA_CHACHA20_POLY1305: u16 = 0xcca8;
    pub const ECDHE_ECDSA_AES256_CBC_SHA: u16 = 0xc00a;
    pub const ECDHE_ECDSA_AES128_CBC_SHA: u16 = 0xc009;
    pub const ECDHE_RSA_AES128_CBC_SHA: u16 = 0xc013;
    pub const ECDHE_RSA_AES256_CBC_SHA: u16 = 0xc014;
    pub const RSA_AES128_GCM_SHA256: u16 = 0x009c;
    pub const RSA_AES256_GCM_SHA384: u16 = 0x009d;
    pub const RSA_AES128_CBC_SHA: u16 = 0x002f;
    pub const RSA_AES256_CBC_SHA: u16 = 0x0035;
    pub const ECDHE_ECDSA_3DES_EDE_CBC_SHA: u16 = 0xc008;
    pub const ECDHE_RSA_3DES_EDE_CBC_SHA: u16 = 0xc012;
    pub const RSA_3DES_EDE_CBC_SHA: u16 = 0x000a;
}

pub mod sigalg {
    pub const ECDSA_SECP256R1_SHA256: u16 = 0x0403;
    pub const ECDSA_SECP384R1_SHA384: u16 = 0x0503;
    pub const ECDSA_SHA1: u16 = 0x0203;
    pub const RSA_PSS_RSAE_SHA256: u16 = 0x0804;
    pub const RSA_PSS_RSAE_SHA384: u16 = 0x0805;
    pub const RSA_PSS_RSAE_SHA512: u16 = 0x0806;
    pub const RSA_PKCS1_SHA256: u16 = 0x0401;
    pub const RSA_PKCS1_SHA384: u16 = 0x0501;
    pub const RSA_PKCS1_SHA512: u16 = 0x0601;
    pub const RSA_PKCS1_SHA1: u16 = 0x0201;
    pub const ML_DSA_44: u16 = 0x0904;
    pub const ML_DSA_65: u16 = 0x0905;
    pub const ML_DSA_87: u16 = 0x0906;
}

pub mod group {
    pub const X25519_MLKEM768: u16 = 0x11ec;
    pub const X25519: u16 = 0x001d;
    pub const SECP256R1: u16 = 0x0017;
    pub const SECP384R1: u16 = 0x0018;
    pub const SECP521R1: u16 = 0x0019;
}

pub mod version {
    pub const TLS1_3: u16 = 0x0304;
    pub const TLS1_2: u16 = 0x0303;
    pub const TLS1_1: u16 = 0x0302;
    pub const TLS1_0: u16 = 0x0301;
}

/// RFC 8879
pub mod cert_compression {
    pub const ZLIB: u16 = 1;
    pub const BROTLI: u16 = 2;
    pub const ZSTD: u16 = 3;
}

// chrome 157 root store trust anchor ids, 8-bit length-prefixed; the same in every captured ClientHello
#[rustfmt::skip]
const CHROME_TRUST_ANCHORS: &[u8] = &[
    0x05, 0x82, 0xdf, 0x13, 0x02, 0x01,
    0x05, 0x82, 0xdf, 0x13, 0x02, 0x06,
    0x05, 0x82, 0xdf, 0x13, 0x02, 0x0d,
    0x05, 0x82, 0xdf, 0x13, 0x02, 0x0e,
    0x05, 0x82, 0xdf, 0x13, 0x02, 0x0f,
    0x05, 0x82, 0xdf, 0x13, 0x02, 0x12,
    0x05, 0x82, 0xdf, 0x13, 0x02, 0x13,
    0x05, 0x82, 0xdf, 0x13, 0x02, 0x14,
    0x08, 0x83, 0x9a, 0x64, 0x8c, 0x9b, 0x2d, 0x01, 0x07,
    0x08, 0x83, 0x9a, 0x64, 0x8c, 0x9b, 0x2d, 0x01, 0x08,
    0x08, 0x83, 0x9a, 0x64, 0x8c, 0x9b, 0x2d, 0x01, 0x09,
    0x08, 0x83, 0x9a, 0x64, 0x8c, 0x9b, 0x2d, 0x01, 0x0a,
    0x08, 0x83, 0x9a, 0x64, 0x8c, 0x9b, 0x2d, 0x01, 0x0b,
    0x08, 0x83, 0x9a, 0x64, 0x8c, 0x9b, 0x2d, 0x01, 0x0c,
    0x08, 0x83, 0x9a, 0x64, 0x8c, 0x9b, 0x2d, 0x01, 0x0d,
    0x08, 0x83, 0x9a, 0x64, 0x8c, 0x9b, 0x2d, 0x01, 0x12,
    0x08, 0x83, 0x9a, 0x64, 0x8c, 0x9b, 0x2d, 0x01, 0x13,
    0x04, 0xd6, 0x79, 0x09, 0x01,
    0x04, 0xd6, 0x79, 0x09, 0x04,
    0x04, 0xd6, 0x79, 0x09, 0x05,
    0x04, 0xd6, 0x79, 0x09, 0x06,
    0x04, 0xd6, 0x79, 0x09, 0x07,
    0x04, 0xd6, 0x79, 0x09, 0x08,
    0x04, 0xd6, 0x79, 0x09, 0x0a,
    0x04, 0xd6, 0x79, 0x09, 0x0b,
    0x04, 0xd6, 0x79, 0x09, 0x0c,
    0x04, 0xd6, 0x79, 0x09, 0x0d,
    0x04, 0xd6, 0x79, 0x09, 0x0f,
];

/// empty lists keep BoringSSL's defaults; GREASE values come from BoringSSL when `grease`
/// is on, not from these lists
///
/// exact cipher and version lists, `sigalg_grease` and `padding` need
/// `deps/patches/boringssl.patch`
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TlsConfig {
    pub cipher_suites: Vec<u16>,
    pub signature_algorithms: Vec<u16>,
    pub groups: Vec<u16>,
    pub supported_versions: Vec<u16>,
    pub cert_compression: Vec<u16>,
    pub alpn: Vec<String>,
    /// ALPS (application_settings) protocol, e.g. `h2`
    pub alps: Option<String>,
    pub new_alps_codepoint: bool,
    pub grease: bool,
    /// GREASE in signature_algorithms when `grease` is on; Chrome yes, Safari no
    pub sigalg_grease: bool,
    pub permute_extensions: bool,
    pub ech_grease: bool,
    pub session_tickets: bool,
    pub ocsp_stapling: bool,
    pub signed_cert_timestamps: bool,
    /// trust_anchors extension with these ids (8-bit length-prefixed); empty sends it empty
    pub trust_anchors: Option<Vec<u8>>,
    /// pads a 256..=511 byte ClientHello to 512 (RFC 7685), like Safari
    pub padding: bool,
}

impl TlsConfig {
    /// Chrome 157, from a capture (the same on Windows and macOS)
    pub fn chrome() -> Self {
        use cipher::*;
        use sigalg::*;

        Self {
            cipher_suites: vec![
                TLS_AES_128_GCM_SHA256,
                TLS_AES_256_GCM_SHA384,
                TLS_CHACHA20_POLY1305_SHA256,
                ECDHE_ECDSA_AES128_GCM_SHA256,
                ECDHE_RSA_AES128_GCM_SHA256,
                ECDHE_ECDSA_AES256_GCM_SHA384,
                ECDHE_RSA_AES256_GCM_SHA384,
                ECDHE_ECDSA_CHACHA20_POLY1305,
                ECDHE_RSA_CHACHA20_POLY1305,
                ECDHE_RSA_AES128_CBC_SHA,
                ECDHE_RSA_AES256_CBC_SHA,
                RSA_AES128_GCM_SHA256,
                RSA_AES256_GCM_SHA384,
                RSA_AES128_CBC_SHA,
                RSA_AES256_CBC_SHA,
            ],
            signature_algorithms: vec![
                ML_DSA_44,
                ML_DSA_65,
                ML_DSA_87,
                ECDSA_SECP256R1_SHA256,
                RSA_PSS_RSAE_SHA256,
                RSA_PKCS1_SHA256,
                ECDSA_SECP384R1_SHA384,
                RSA_PSS_RSAE_SHA384,
                RSA_PKCS1_SHA384,
                RSA_PSS_RSAE_SHA512,
                RSA_PKCS1_SHA512,
            ],
            groups: vec![
                group::X25519_MLKEM768,
                group::X25519,
                group::SECP256R1,
                group::SECP384R1,
            ],
            supported_versions: vec![version::TLS1_3, version::TLS1_2],
            cert_compression: vec![cert_compression::BROTLI],
            alpn: vec!["h2".into(), "http/1.1".into()],
            alps: Some("h2".into()),
            new_alps_codepoint: true,
            grease: true,
            sigalg_grease: true,
            permute_extensions: true,
            ech_grease: true,
            session_tickets: true,
            ocsp_stapling: true,
            signed_cert_timestamps: true,
            trust_anchors: Some(CHROME_TRUST_ANCHORS.to_vec()),
            padding: false,
        }
    }

    /// Safari 27 (iOS 27), from an iphone capture
    pub fn safari_ios() -> Self {
        use cipher::*;
        use sigalg::*;

        Self {
            // tls 1.3 suites in apple's order: aes-256 first
            cipher_suites: vec![
                TLS_AES_256_GCM_SHA384,
                TLS_CHACHA20_POLY1305_SHA256,
                TLS_AES_128_GCM_SHA256,
                ECDHE_ECDSA_AES256_GCM_SHA384,
                ECDHE_ECDSA_AES128_GCM_SHA256,
                ECDHE_ECDSA_CHACHA20_POLY1305,
                ECDHE_RSA_AES256_GCM_SHA384,
                ECDHE_RSA_AES128_GCM_SHA256,
                ECDHE_RSA_CHACHA20_POLY1305,
                ECDHE_ECDSA_AES256_CBC_SHA,
                ECDHE_ECDSA_AES128_CBC_SHA,
                ECDHE_RSA_AES256_CBC_SHA,
                ECDHE_RSA_AES128_CBC_SHA,
                RSA_AES256_GCM_SHA384,
                RSA_AES128_GCM_SHA256,
                RSA_AES256_CBC_SHA,
                RSA_AES128_CBC_SHA,
                ECDHE_ECDSA_3DES_EDE_CBC_SHA,
                ECDHE_RSA_3DES_EDE_CBC_SHA,
                RSA_3DES_EDE_CBC_SHA,
            ],
            // Safari really lists rsa_pss_rsae_sha384 twice
            signature_algorithms: vec![
                ECDSA_SECP256R1_SHA256,
                RSA_PSS_RSAE_SHA256,
                RSA_PKCS1_SHA256,
                ECDSA_SECP384R1_SHA384,
                RSA_PSS_RSAE_SHA384,
                RSA_PSS_RSAE_SHA384,
                RSA_PKCS1_SHA384,
                RSA_PSS_RSAE_SHA512,
                RSA_PKCS1_SHA512,
                RSA_PKCS1_SHA1,
            ],
            groups: vec![
                group::X25519_MLKEM768,
                group::X25519,
                group::SECP256R1,
                group::SECP384R1,
                group::SECP521R1,
            ],
            supported_versions: vec![version::TLS1_3, version::TLS1_2],
            cert_compression: vec![cert_compression::ZLIB],
            alpn: vec!["h2".into(), "http/1.1".into()],
            alps: None,
            new_alps_codepoint: false,
            grease: true,
            sigalg_grease: false,
            permute_extensions: false,
            ech_grease: false,
            session_tickets: false,
            ocsp_stapling: true,
            signed_cert_timestamps: true,
            trust_anchors: None,
            padding: true,
        }
    }
}

// wire format: each name prefixed with its length
pub(crate) fn encode_alpn(protocols: &[String]) -> Vec<u8> {
    protocols
        .iter()
        .flat_map(|protocol| {
            let len = u8::try_from(protocol.len()).expect("ALPN protocol name over 255 bytes");
            std::iter::once(len).chain(protocol.bytes())
        })
        .collect()
}

/// cheap to clone and share between connections
#[derive(Clone)]
pub struct TlsPreset {
    pub(crate) ptr: SharedPtr<ffi::ssl_preset>,
}

impl TlsPreset {
    pub fn new(config: &TlsConfig) -> Self {
        let mut preset = ffi::new_ssl_preset();
        let mut p = preset.pin_mut();

        p.as_mut().set_permute_extensions(config.permute_extensions);
        p.as_mut().set_enable_grease(config.grease);
        p.as_mut().set_enable_sigalg_grease(config.sigalg_grease);
        p.as_mut().set_enable_padding(config.padding);
        p.as_mut().set_enable_ech_grease(config.ech_grease);
        p.as_mut().set_enable_session_ticket(config.session_tickets);
        p.as_mut().set_enable_ocsp_stapling(config.ocsp_stapling);
        p.as_mut()
            .set_enable_signed_cert_timestamps(config.signed_cert_timestamps);
        p.as_mut()
            .set_enable_new_alps_codepoint(config.new_alps_codepoint);

        if let Some(ids) = &config.trust_anchors {
            p.as_mut().set_trust_anchors(ids.clone());
        }

        for &alg in &config.cert_compression {
            p.as_mut().add_cert_compression_alg(alg);
        }
        for &cipher in &config.cipher_suites {
            p.as_mut().add_cipher_suite(cipher);
        }
        for &alg in &config.signature_algorithms {
            p.as_mut().add_verify_algorithm_pref(alg);
        }
        for &group in &config.groups {
            p.as_mut().add_group_id(group);
        }
        for &version in &config.supported_versions {
            p.as_mut().add_supported_version(version);
        }

        if !config.alpn.is_empty() {
            p.as_mut().set_alpn_protos(encode_alpn(&config.alpn));
        }
        if let Some(alps) = &config.alps {
            p.as_mut()
                .set_application_settings(alps.as_bytes().to_vec());
        }

        Self {
            ptr: ffi::share_ssl_preset(preset),
        }
    }

    pub fn chrome() -> Self {
        Self::new(&TlsConfig::chrome())
    }

    pub fn safari_ios() -> Self {
        Self::new(&TlsConfig::safari_ios())
    }
}
