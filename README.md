# mhttp

An HTTP/1.1 and HTTP/2 client whose TLS ClientHello and HTTP/2 connection preface are fully
configurable, so its traffic matches a chosen browser on the wire. The engine is C++
(BoringSSL, nghttp2, libuv); callers use a small async Rust API.

```rust
use mhttp::{Connection, CookiePlacement, Profile, Request};

let connection = Connection::builder("https://tls.peet.ws".parse()?)
    .profile(&Profile::safari_ios()) // Chrome is the default
    .connect()
    .await?;

let request = Request::get("/api/all")
    .header("accept", "application/json")
    .cookies(CookiePlacement::Last)
    .build();
let response = connection.send(request).await?;
println!("{} {}", response.status(), response.text());
```

## Profiles

A `Profile` sets the ClientHello, the HTTP/2 preface, HPACK encoding, the pseudo-header order and
the default stream priority together. The values come from captured browser traffic.

| | Chrome 157 (captured on macOS) | Safari 27 (iOS 27, captured from an iPhone) |
|---|---|---|
| Key exchange | X25519MLKEM768, X25519, P-256, P-384 | X25519MLKEM768, X25519, P-256, P-384, P-521 |
| Signature algorithms | GREASE, ML-DSA-44/65/87, then the usual ECDSA/RSA set | no GREASE; `rsa_pss_rsae_sha384` twice, as Safari sends it |
| Extensions | permuted per connection, GREASE, ECH GREASE, ALPS (new codepoint), brotli certificate compression, `trust_anchors` with Chrome's 28 root store ids | fixed order, GREASE, zlib certificate compression, no session ticket; TLS 1.3 suites AES-256 first |
| HTTP/2 SETTINGS | `1:65536, 2:0, 4:6291456, 6:262144` | `2:0, 3:100, 4:2097152, 9:1` |
| WINDOW_UPDATE | 15663105 | 10420225 |
| Pseudo-headers | `:method :authority :scheme :path` | `:method :scheme :authority :path` |
| Stream priority | exclusive on stream 0, weight 256 | none (RFC 7540 priorities off) |
| First write after the handshake | preface, SETTINGS, WINDOW_UPDATE | the same plus the first request, in one TLS record |
| HPACK | every field indexed except pseudo-headers other than `:authority` | the same, cookies included |

What [tls.peet.ws](https://tls.peet.ws/api/all) sees (`cargo run --example fingerprint`):

| | JA4 | HTTP/2 fingerprint |
|---|---|---|
| Chrome, h2 | `t13d1517h2_8daaf6152771_cb7bf5808d99` (same as Chrome 157) | `1:65536;2:0;4:6291456;6:262144\|15663105\|0\|m,a,s,p` |
| Safari, h2 | `t13d2013h2_a09f3c656075_7f0f34a4126d` (same as the iPhone) | `2:0;3:100;4:2097152;9:1\|10420225\|0\|m,s,a,p` |

Everything is plain data in `TlsConfig` and `Http2Config`, so a profile can be copied and changed,
or a new one written from a capture.

## Rust API

- `Connection`: one connection to one origin. HTTP/2 requests can be sent concurrently from any
  number of tasks; a request that times out resets only its own stream. Optional HTTP CONNECT
  proxy (`Proxy`), a cookie jar that can be shared between connections, a logger that receives
  every request and response, and `connect_to` to open the TCP connection to a specific edge node
  while SNI and headers keep the URL's host.
- `Request`: header order is kept exactly. HTTP/2 pseudo-headers and the HTTP/1.1 `Host` header
  are generated when missing; jar cookies go first, last or after a named header, with a chosen
  cookie order, and as separate `cookie` headers on HTTP/2 like browsers send them.
- `Response`: headers in arrival order; the body decoded from gzip, deflate, brotli or zstd.

`src/bridge.rs` is the only module with `unsafe` code: the cxx bridge and the glue that turns the
engine's coroutines into Rust futures.

## Engine

- **Threading.** Each connection runs a libuv loop on its own thread, and all its state lives on
  that thread. Calls from Rust start on the caller's thread and hop onto the loop through one
  queue before touching anything; coroutines are resumed through the same queue, never from
  inside a libuv or nghttp2 callback.
- **TLS: BoringSSL.** Every ClientHello setting is a field of `ssl_preset` applied in
  `ssl_manager::modify_session`; a setting BoringSSL rejects fails the connection instead of
  silently falling back to a default ClientHello. Certificate compression (zlib, brotli, zstd) is
  decoded on the Rust side.
- **HTTP/2: nghttp2.** SETTINGS and their order, the connection WINDOW_UPDATE, optional PRIORITY
  frames, the per-request priority and per-field HPACK choices. The preface leaves in one write,
  as browsers send it.
- **HTTP/1.1:** own request serialization; responses parsed with Node's `http_parser`, including
  bodies delimited by the connection closing.
- **I/O: libuv**, TLS over libuv through a modified [uvtls](https://github.com/mpenick/uvtls) in
  `native/uvtls/`: cut down to the client side, without the OpenSSL 1.0 and LibreSSL compatibility
  code, with `uvtls_get_ssl` so the ClientHello can be configured before the handshake, and
  without the handshake-write assertion that aborted the process on a connection reset.
- **TLS key logging:** with `SSLKEYLOGFILE` set, session secrets are written in the NSS key log
  format, so Wireshark can decrypt a capture and compare it with a real browser's.

## Patched dependencies

Stock libraries can't produce a browser's exact handshake, so three are patched
(`deps/patches/`, described in `deps/info.txt`):

- **BoringSSL:** write the cipher suite and supported_versions lists exactly as given, turn the
  signature-algorithm GREASE off on its own, RFC 7685 padding, repeated verify preferences.
- **nghttp2:** RFC 7540 priorities in HEADERS and PRIORITY frames (upstream dropped them, Chrome
  still sends them) and per-field HPACK flags for indexing and Huffman coding.
- **cxx-async:** the promise's channel is built through an explicit pointer, which fixes a
  crash on arm64 (Apple Silicon), plus small GCC and MSVC fixes.

## Building

```
eval "$(deps/build.sh)"   # Linux/macOS: fetch, patch and build the dependencies (~1 min)
. deps/build.ps1          # Windows, from a Visual Studio developer prompt
cargo build
```

`deps/info.txt` has the pinned versions. Without the dependencies only the Rust side compiles.
CI builds and runs the wire tests on Linux, macOS and Windows.

## Tests

- `cargo test`: with the engine built, wire tests: a minimal HTTP/2 server reads the raw frames
  and HPACK representations the engine sends.
- `cargo test --test live -- --ignored`: requests to real servers, including 40 concurrent
  HTTP/2 requests on one connection, request bodies over one frame, per-stream timeouts and
  dropping a connection with requests in flight. They also pass with the engine built under
  AddressSanitizer and LeakSanitizer.

## Known limits

- Server certificates are not verified (`verify_cb` in `native/ssl_manager.cpp`): acceptable for
  a research client, not for anything production-facing.
- One HTTP/1.1 request at a time per connection; no pipelining.
- Only HTTP CONNECT proxies, no SOCKS.

## License

MIT. `native/http_parser.*` is Node's HTTP parser (MIT, Joyent and Node contributors) and
`native/uvtls/` is uvtls (MIT, Michael A. Penick); both keep their original notices.
