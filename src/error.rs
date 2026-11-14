#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{message} ({code})")]
    Native { code: i32, message: &'static str },

    #[error(transparent)]
    Exception(#[from] cxx_async::CxxAsyncException),

    #[error("decoding the response body: {0}")]
    Decode(#[from] std::io::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error("invalid cookie: {0}")]
    Cookie(#[from] cookie::ParseError),

    #[error("invalid request: {0}")]
    InvalidRequest(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
