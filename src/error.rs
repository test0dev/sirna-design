use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Msg(String),

    #[error("HTTP request failed: {0}")]
    Http(String),

    #[error("Ensembl HTTP {status}: {message}")]
    EnsemblHttp { status: u16, message: String },
}
