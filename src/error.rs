use thiserror::Error;

#[derive(Debug, Error)]
pub enum TranslateError {
    #[error("network request failed: {0}")]
    Network(String),

    #[error("failed to parse Bing anti-abuse token: {0}")]
    TokenParse(String),

    #[error("translation request rejected (status {0})")]
    ApiRejected(i64),

    #[error("unexpected response format: {0}")]
    Malformed(String),

    #[error("translation provider failed: {0}")]
    Provider(String),

    #[error("input text is empty")]
    Empty,

    #[error("translation result is empty")]
    NoResult,

    #[error("failed to read from standard input")]
    Stdin,

    #[error("no text provided")]
    NoInput,
}