//! What the engine puts on the wire, read back by a minimal HTTP/2 server (cleartext, prior
//! knowledge) that records frames and HPACK representations instead of decoding headers.
#![cfg(mhttp_engine)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use mhttp::preset::{
    HpackRule, Http2Config, Http2Preset, Huffman, Literal, PriorityFrame, TlsPreset,
};
use mhttp::{Connection, Priority, Profile, Pseudo, Request};

#[derive(Debug, Clone, PartialEq)]
enum Repr {
    Indexed(usize),
    Literal {
        kind: Literal,
        // static or dynamic table index of the name, or the literal name
        name: Result<usize, String>,
        name_huffman: Option<bool>,
        value_huffman: bool,
    },
}

#[derive(Debug, Default)]
struct Seen {
    settings: Vec<(u16, u32)>,
    window_update: Option<u32>,
    priority_frames: Vec<(u32, u32, bool, u8)>,
    // (exclusive, depends_on, wire weight)
    headers_priority: Option<(bool, u32, u8)>,
    fields: Vec<Repr>,
}

fn integer(block: &[u8], pos: &mut usize, prefix: u8) -> usize {
    let mask = (1u16 << prefix) as u8 - 1;
    let mut value = (block[*pos] & mask) as usize;
    *pos += 1;
    if value < mask as usize {
        return value;
    }
    let mut shift = 0;
    loop {
        let byte = block[*pos];
        *pos += 1;
        value += ((byte & 0x7f) as usize) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            return value;
        }
    }
}

// Returns the Huffman bit and the raw (possibly encoded) bytes.
fn string(block: &[u8], pos: &mut usize) -> (bool, Vec<u8>) {
    let huffman = block[*pos] & 0x80 != 0;
    let len = integer(block, pos, 7);
    let bytes = block[*pos..*pos + len].to_vec();
    *pos += len;
    (huffman, bytes)
}

fn hpack(block: &[u8]) -> Vec<Repr> {
    let mut pos = 0;
    let mut out = Vec::new();
    while pos < block.len() {
        let first = block[pos];
        if first & 0x80 != 0 {
            out.push(Repr::Indexed(integer(block, &mut pos, 7)));
            continue;
        }
        if first & 0xe0 == 0x20 {
            integer(block, &mut pos, 5); // table size update
            continue;
        }
        let (kind, prefix) = match first {
            b if b & 0xc0 == 0x40 => (Literal::Indexed, 6),
            b if b & 0xf0 == 0x10 => (Literal::NeverIndexed, 4),
            _ => (Literal::NotIndexed, 4),
        };
        let index = integer(block, &mut pos, prefix);
        let (name, name_huffman) = if index == 0 {
            let (h, bytes) = string(block, &mut pos);
            (Err(String::from_utf8_lossy(&bytes).into_owned()), Some(h))
        } else {
            (Ok(index), None)
        };
        let (value_huffman, _) = string(block, &mut pos);
        out.push(Repr::Literal {
            kind,
            name,
            name_huffman,
            value_huffman,
        });
    }
    out
}

// Serves one request on one connection and returns what the client sent.
fn serve_once() -> (u16, thread::JoinHandle<Seen>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let handle = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut seen = Seen::default();

        let mut preface = [0u8; 24];
        socket.read_exact(&mut preface).unwrap();
        assert_eq!(&preface, b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n");

        // Our SETTINGS, then answer the first request.
        socket.write_all(&[0, 0, 0, 4, 0, 0, 0, 0, 0]).unwrap();

        loop {
            let mut header = [0u8; 9];
            socket.read_exact(&mut header).unwrap();
            let len = u32::from_be_bytes([0, header[0], header[1], header[2]]) as usize;
            let (kind, flags) = (header[3], header[4]);
            let stream =
                u32::from_be_bytes([header[5], header[6], header[7], header[8]]) & 0x7fff_ffff;
            let mut payload = vec![0u8; len];
            socket.read_exact(&mut payload).unwrap();

            match kind {
                4 if flags & 1 == 0 => {
                    for chunk in payload.chunks(6) {
                        let id = u16::from_be_bytes([chunk[0], chunk[1]]);
                        let value = u32::from_be_bytes([chunk[2], chunk[3], chunk[4], chunk[5]]);
                        seen.settings.push((id, value));
                    }
                    socket.write_all(&[0, 0, 0, 4, 1, 0, 0, 0, 0]).unwrap();
                }
                8 if stream == 0 => {
                    seen.window_update = Some(u32::from_be_bytes([
                        payload[0], payload[1], payload[2], payload[3],
                    ]));
                }
                2 => {
                    let dep = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                    seen.priority_frames.push((
                        stream,
                        dep & 0x7fff_ffff,
                        dep >> 31 == 1,
                        payload[4],
                    ));
                }
                1 => {
                    let mut block = &payload[..];
                    if flags & 0x20 != 0 {
                        let dep = u32::from_be_bytes([block[0], block[1], block[2], block[3]]);
                        seen.headers_priority = Some((dep >> 31 == 1, dep & 0x7fff_ffff, block[4]));
                        block = &block[5..];
                    }
                    seen.fields = hpack(block);

                    // :status 200, end of stream.
                    let mut response = vec![0, 0, 1, 1, 0x05];
                    response.extend_from_slice(&stream.to_be_bytes());
                    response.push(0x88);
                    socket.write_all(&response).unwrap();

                    // drain until the client hangs up; closing with unread bytes would reset the connection
                    let _ = std::io::copy(&mut socket, &mut std::io::sink());
                    return seen;
                }
                _ => {}
            }
        }
    });

    (port, handle)
}

async fn request_through(profile: &Profile, port: u16) {
    let url = format!("http://127.0.0.1:{port}");
    let connection = Connection::builder(url.parse().unwrap())
        .profile(profile)
        .connect()
        .await
        .unwrap();
    let request = Request::get("/x").header("x-test", "1").build();
    assert_eq!(connection.send(request).await.unwrap().status(), 200);
}

#[tokio::test]
async fn chrome_preface_priority_and_hpack() {
    let (port, server) = serve_once();
    request_through(&Profile::chrome(), port).await;
    let seen = server.join().unwrap();

    assert_eq!(
        seen.settings,
        [(1, 65536), (2, 0), (4, 6291456), (6, 262144)]
    );
    assert_eq!(seen.window_update, Some(15663105));
    // exclusive on stream 0, weight 256 (255 on the wire)
    assert_eq!(seen.headers_priority, Some((true, 0, 255)));

    assert_eq!(seen.fields[0], Repr::Indexed(2)); // :method GET
    assert!(matches!(
        seen.fields[1],
        Repr::Literal {
            kind: Literal::Indexed,
            name: Ok(1),
            ..
        }
    )); // :authority
    assert_eq!(seen.fields[2], Repr::Indexed(6)); // :scheme http
    assert!(matches!(
        seen.fields[3],
        Repr::Literal {
            kind: Literal::NotIndexed,
            name: Ok(4),
            ..
        }
    )); // :path
    assert!(matches!(
        seen.fields[4],
        Repr::Literal {
            kind: Literal::Indexed,
            name: Err(_),
            name_huffman: Some(true),
            ..
        }
    )); // x-test
}

#[tokio::test]
async fn safari_sends_no_priority() {
    let (port, server) = serve_once();
    request_through(&Profile::safari_ios(), port).await;
    let seen = server.join().unwrap();

    assert_eq!(seen.settings, [(2, 0), (3, 100), (4, 2097152), (9, 1)]);
    assert_eq!(seen.window_update, Some(10420225));
    assert_eq!(seen.headers_priority, None);
    // :method :scheme :authority :path
    assert_eq!(seen.fields[0], Repr::Indexed(2));
    assert_eq!(seen.fields[1], Repr::Indexed(6));
}

#[tokio::test]
async fn hpack_rules_huffman_and_priority_frames_are_honoured() {
    let config = Http2Config {
        settings: vec![(4, 65535)],
        window_update: Some(1000),
        hpack: vec![
            HpackRule::new("x-test", Literal::NeverIndexed),
            HpackRule::new("*", Literal::NotIndexed),
        ],
        huffman: Huffman::Never,
        priority_frames: vec![PriorityFrame {
            stream_id: 3,
            priority: Priority {
                depends_on: 0,
                weight: 201,
                exclusive: false,
            },
        }],
        preface_with_first_request: false,
    };
    let profile = Profile {
        tls: TlsPreset::chrome(),
        http2: Http2Preset::new(&config),
        pseudo_order: Some(Pseudo::CHROME),
        priority: None,
    };

    let (port, server) = serve_once();
    request_through(&profile, port).await;
    let seen = server.join().unwrap();

    assert_eq!(seen.priority_frames, [(3, 0, false, 200)]);
    assert_eq!(seen.headers_priority, None);
    assert_eq!(
        seen.fields[1],
        Repr::Literal {
            kind: Literal::NotIndexed,
            name: Ok(1),
            name_huffman: None,
            value_huffman: false
        }
    );
    assert_eq!(
        seen.fields[4],
        Repr::Literal {
            kind: Literal::NeverIndexed,
            name: Err("x-test".into()),
            name_huffman: Some(false),
            value_huffman: false
        }
    );
}
