use cxx::SharedPtr;

use crate::bridge::ffi;
use crate::request::Priority;

pub mod setting {
    pub const HEADER_TABLE_SIZE: u16 = 1;
    pub const ENABLE_PUSH: u16 = 2;
    pub const MAX_CONCURRENT_STREAMS: u16 = 3;
    pub const INITIAL_WINDOW_SIZE: u16 = 4;
    pub const MAX_FRAME_SIZE: u16 = 5;
    pub const MAX_HEADER_LIST_SIZE: u16 = 6;
    pub const NO_RFC7540_PRIORITIES: u16 = 9;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Literal {
    Indexed,
    NotIndexed,
    NeverIndexed,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Huffman {
    #[default]
    Shortest,
    Always,
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HpackRule {
    // a header name, ":*" for any pseudo-header, "*" for any header
    pub name: String,
    pub literal: Literal,
}

impl HpackRule {
    pub fn new(name: &str, literal: Literal) -> Self {
        Self {
            name: name.to_string(),
            literal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriorityFrame {
    pub stream_id: i32,
    pub priority: Priority,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Http2Config {
    pub settings: Vec<(u16, u32)>,
    pub window_update: Option<u32>,
    // first matching rule wins; headers no rule matches get nghttp2's own choice
    pub hpack: Vec<HpackRule>,
    pub huffman: Huffman,
    // sent right after the preface, as some older browsers did
    pub priority_frames: Vec<PriorityFrame>,
    // hold the preface back so it shares a tls record with the first request, like safari
    pub preface_with_first_request: bool,
}

impl Http2Config {
    /// Chrome 157, from a capture
    pub fn chrome() -> Self {
        use setting::*;

        Self {
            settings: vec![
                (HEADER_TABLE_SIZE, 65536),
                (ENABLE_PUSH, 0),
                (INITIAL_WINDOW_SIZE, 6291456),
                (MAX_HEADER_LIST_SIZE, 262144),
            ],
            window_update: Some(15663105),
            // all indexed but pseudo-headers other than :authority (captured, matches QUICHE's encoder)
            hpack: vec![
                HpackRule::new(":authority", Literal::Indexed),
                HpackRule::new(":*", Literal::NotIndexed),
                HpackRule::new("*", Literal::Indexed),
            ],
            huffman: Huffman::Shortest,
            priority_frames: vec![],
            preface_with_first_request: false,
        }
    }

    /// Safari 27 (iOS 27), from an iphone capture
    pub fn safari_ios() -> Self {
        use setting::*;

        Self {
            settings: vec![
                (ENABLE_PUSH, 0),
                (MAX_CONCURRENT_STREAMS, 100),
                (INITIAL_WINDOW_SIZE, 2097152),
                (NO_RFC7540_PRIORITIES, 1),
            ],
            window_update: Some(10420225),
            // same policy as chrome: cookies indexed too
            hpack: vec![
                HpackRule::new(":authority", Literal::Indexed),
                HpackRule::new(":*", Literal::NotIndexed),
                HpackRule::new("*", Literal::Indexed),
            ],
            huffman: Huffman::Shortest,
            priority_frames: vec![],
            preface_with_first_request: true,
        }
    }
}

#[derive(Clone)]
pub struct Http2Preset {
    pub(crate) ptr: SharedPtr<ffi::http2_preset>,
}

impl Http2Preset {
    pub fn new(config: &Http2Config) -> Self {
        let mut preset = ffi::new_http2_preset();
        let mut p = preset.pin_mut();

        for &(id, value) in &config.settings {
            p.as_mut().settings_add(i32::from(id), value);
        }

        if let Some(increment) = config.window_update {
            let increment = i32::try_from(increment).expect("WINDOW_UPDATE increment over 2^31-1");
            p.as_mut().set_window_update(increment);
        }

        for rule in &config.hpack {
            let mode = match rule.literal {
                Literal::Indexed => 0,
                Literal::NotIndexed => 1,
                Literal::NeverIndexed => 2,
            };
            p.as_mut().add_hpack_rule(&rule.name, mode);
        }

        p.as_mut().set_hpack_huffman(match config.huffman {
            Huffman::Shortest => 0,
            Huffman::Always => 1,
            Huffman::Never => 2,
        });

        p.as_mut()
            .set_preface_with_first_request(config.preface_with_first_request);

        for frame in &config.priority_frames {
            p.as_mut().add_priority_frame(
                frame.stream_id,
                frame.priority.depends_on,
                frame.priority.weight,
                frame.priority.exclusive,
            );
        }

        Self {
            ptr: ffi::share_http2_preset(preset),
        }
    }

    pub fn chrome() -> Self {
        Self::new(&Http2Config::chrome())
    }

    pub fn safari_ios() -> Self {
        Self::new(&Http2Config::safari_ios())
    }
}
