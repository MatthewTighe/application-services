use std::num::{ParseIntError};

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Error sending request: {0}")]
    RequestError(#[from] viaduct::Error),
    #[error("Error parsing URL: {0}")]
    UrlParsingError(#[from] url::ParseError),
    #[error("Server asked the client to back off ({0} seconds remaining)")]
    BackoffError(u64),
    #[error("Error in network response: {0}")]
    ResponseError(String),
    // #[error("ParseIntError: {0}")]
    // ParseIntError(#[from] ParseIntError),
}

pub type Result<T, E = ClientError> = std::result::Result<T, E>;