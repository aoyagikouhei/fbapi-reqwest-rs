use thiserror::Error;

#[derive(Debug, Error)]
pub enum FbapiError {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error("facebook error: {0}")]
    Facebook(serde_json::Value),
}
