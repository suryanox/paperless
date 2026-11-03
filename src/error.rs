use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("pdf parse error: {0}")]
    Pdf(String),

    #[error("extractor panicked on malformed pdf")]
    Panicked,
}
