use std::borrow::Cow;
use std::io::Read;

use crate::Result;

/// body already decompressed
#[derive(Debug, Clone)]
pub struct Response {
    status: u16,
    reason: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Response {
    // headers one `name: value` per line, body still content-encoded
    pub(crate) fn from_raw(
        status: u32,
        reason: String,
        raw_headers: &str,
        body: Vec<u8>,
    ) -> Result<Self> {
        let headers = parse_headers(raw_headers);
        let encoding = headers
            .iter()
            .find(|(name, _)| name == "content-encoding")
            .map(|(_, value)| value.as_str());
        let body = decode_body(encoding, body)?;

        Ok(Self {
            status: u16::try_from(status).unwrap_or(0),
            reason,
            headers,
            body,
        })
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// arrival order, names lowercase
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// first value
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn header_all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.headers
            .iter()
            .filter(move |(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// lossy UTF-8
    pub fn text(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.body)
    }
}

fn parse_headers(raw: &str) -> Vec<(String, String)> {
    raw.lines()
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect()
}

// last applied first: `gzip, br` takes brotli off first
fn decode_body(encoding: Option<&str>, body: Vec<u8>) -> std::io::Result<Vec<u8>> {
    let Some(encoding) = encoding else {
        return Ok(body);
    };

    let mut body = body;
    for coding in encoding.split(',').rev().map(str::trim) {
        if body.is_empty() {
            break;
        }

        let mut decoded = Vec::new();
        match coding.to_ascii_lowercase().as_str() {
            "gzip" | "x-gzip" => {
                flate2::read::MultiGzDecoder::new(body.as_slice()).read_to_end(&mut decoded)?;
            }
            // RFC 9110 says zlib-wrapped, some servers send raw deflate
            "deflate" => {
                if flate2::read::ZlibDecoder::new(body.as_slice())
                    .read_to_end(&mut decoded)
                    .is_err()
                {
                    decoded.clear();
                    flate2::read::DeflateDecoder::new(body.as_slice()).read_to_end(&mut decoded)?;
                }
            }
            "br" => {
                brotli::Decompressor::new(body.as_slice(), 4096).read_to_end(&mut decoded)?;
            }
            "zstd" => {
                zstd::stream::read::Decoder::new(body.as_slice())?.read_to_end(&mut decoded)?;
            }
            _ => continue,
        }
        body = decoded;
    }

    Ok(body)
}
