use thiserror::Error;

#[derive(Debug, Error)]
pub enum FbapiError {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error("facebook error: {0}")]
    Facebook(serde_json::Value),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("Faecbook unexpected json: {0}")]
    UnExpected(serde_json::Value),
}
