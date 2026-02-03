pub mod apis;
pub mod batch_request;
pub mod error;

#[macro_use]
extern crate serde_json;

use crate::error::FbapiError;
use crypto::mac::Mac;
use once_cell::sync::Lazy;
use reqwest::{multipart::Part, Body};
use std::{future::Future, time::Duration};
use tokio::time::sleep;

pub use reqwest;

const GRAPH_PREFIX: &'static str = "https://graph.facebook.com/";
const VIDEO_PREFIX: &'static str = "https://graph-video.facebook.com/";
const VIDEO_REEL_URL_PREFIX: &'static str = "https://rupload.facebook.com/video-upload/";

static ERROR_VALUE: Lazy<serde_json::Value> = Lazy::new(|| {
    json!({
        "error": {
            "message": "(#32) Page request limit reached",
            "type": "OAuthException",
            "code": 32,
            "fbtrace_id": "emulated"
        }
    })
});

pub struct Fbapi {
    client: reqwest::Client,
    version: String,
    rate_limit_emulation: bool,
}

impl Fbapi {
    pub fn new(
        version: &str,
        timeout_seconds: u64,
        rate_limit_emulation: bool,
    ) -> Result<Self, FbapiError> {
        Ok(Self {
            client: Self::make_client(timeout_seconds)?,
            version: version.to_owned(),
            rate_limit_emulation: rate_limit_emulation,
        })
    }

    fn make_path(&self, postfix: &str) -> String {
        format!("{}{}/{}", GRAPH_PREFIX, self.version, postfix)
    }

    fn make_video_path(&self, postfix: &str) -> String {
        format!("{}{}/{}", VIDEO_PREFIX, self.version, postfix)
    }

    fn make_video_reel_path(&self, video_id: &str) -> String {
        format!("{}{}/{}", VIDEO_REEL_URL_PREFIX, self.version, video_id)
    }

    pub fn make_client(timeout_seconds: u64) -> Result<reqwest::Client, FbapiError> {
        reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .map_err(|e| e.into())
    }
}

pub(crate) async fn execute_retry<Executor, ResponseFutuer>(
    retry_count: usize,
    executor: Executor,
    log: &impl Fn(LogParams),
    src_params: LogParams,
) -> Result<serde_json::Value, FbapiError>
where
    ResponseFutuer: Future<Output = Result<reqwest::Response, FbapiError>>,
    Executor: Fn() -> ResponseFutuer,
{
    let mut count: usize = 0;
    let mut last_error: FbapiError;
    loop {
        let mut params = src_params.clone();
        params.count = count;
        log(params);
        match executor().await {
            Ok(response) => {
                // Extract rate limit headers before consuming response with json()
                let app_usage = response
                    .headers()
                    .get("x-app-usage")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_owned());
                let business_use_case_usage = response
                    .headers()
                    .get("x-business-use-case-usage")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_owned());

                match response.json::<serde_json::Value>().await {
                    Ok(json) => {
                        if json["error"].is_object() {
                            return Err(FbapiError::Facebook(json));
                        } else {
                            let mut params = src_params.clone();
                            params.count = count;
                            params.result = Some(json.clone());
                            params.app_usage = app_usage;
                            params.business_use_case_usage = business_use_case_usage;
                            log(params);
                            return Ok(json);
                        }
                    }
                    Err(err) => last_error = err.into(),
                }
            }
            Err(err) => last_error = err.into(),
        };
        count = count + 1;
        if count >= retry_count {
            break;
        }
    }
    Err(last_error)
}

pub(crate) async fn execute_form(
    client: &reqwest::Client,
    path: &str,
    form: reqwest::multipart::Form,
    log: &impl Fn(LogParams),
    log_params: LogParams,
) -> Result<serde_json::Value, FbapiError> {
    log(log_params.clone());
    let json: serde_json::Value = client
        .post(path)
        .multipart(form)
        .send()
        .await?
        .json()
        .await?;
    let mut log_params = log_params.clone();
    log_params.result = Some(json.clone());
    log(log_params);
    if json["error"].is_object() {
        Err(FbapiError::Facebook(json))
    } else {
        Ok(json)
    }
}

fn sign(base: &str, key: &str) -> String {
    let mut hmac = crypto::hmac::Hmac::new(crypto::sha2::Sha256::new(), key.as_bytes());
    hmac.input(base.as_bytes());
    hmac.result()
        .code()
        .iter()
        .map(|&x| format!("{:02x}", x))
        .collect()
}

#[derive(Clone)]
pub struct LogParams {
    pub path: String,
    pub params: Vec<(String, String)>,
    pub count: usize,
    pub result: Option<serde_json::Value>,
    pub app_usage: Option<String>,
    pub business_use_case_usage: Option<String>,
}

impl LogParams {
    fn new(path: &str, params: &Vec<(&str, &str)>) -> Self {
        let mut dst = vec![];
        for param in params {
            dst.push((param.0.to_owned(), param.1.to_owned()));
        }
        Self {
            path: path.to_owned(),
            params: dst,
            count: 0,
            result: None,
            app_usage: None,
            business_use_case_usage: None,
        }
    }
}

/// Validates that a media ID is a valid non-zero numeric string.
///
/// Returns an error if the ID is empty, "0", or contains non-digit characters.
pub fn validate_media_id(id: &str, response: &serde_json::Value) -> Result<String, FbapiError> {
    if id.is_empty() || id == "0" || !id.chars().all(|c| c.is_ascii_digit()) {
        return Err(FbapiError::InvalidMediaId {
            id: id.to_owned(),
            response: response.clone(),
        });
    }
    Ok(id.to_owned())
}

pub(crate) fn make_part(path: &str, bytes: rusoto_core::ByteStream) -> Result<Part, FbapiError> {
    Part::stream(Body::wrap_stream(bytes))
        .file_name(path.to_owned())
        .mime_str("application/octet-stream")
        .map_err(|e| e.into())
}

pub(crate) async fn sleep_sec(seconds: usize) {
    sleep(Duration::from_secs(seconds as u64)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let api = Fbapi::new("v8.0", 10, true).unwrap();
        let res = api
            .get_object("xxxx", None, "aaa", "", &vec![], 2, |params| {
                println!(
                    "params {},{:?},{},{:?}",
                    params.path, params.params, params.count, params.result
                )
            })
            .await;
        println!("{:?}", res);
    }

    #[test]
    fn test_validate_media_id_valid() {
        let response = json!({"id": "17841400123456789"});
        let result = validate_media_id("17841400123456789", &response);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "17841400123456789");
    }

    #[test]
    fn test_validate_media_id_zero() {
        let response = json!({"id": "0"});
        let result = validate_media_id("0", &response);
        assert!(result.is_err());
        match result.unwrap_err() {
            FbapiError::InvalidMediaId { id, .. } => assert_eq!(id, "0"),
            _ => panic!("Expected InvalidMediaId error"),
        }
    }

    #[test]
    fn test_validate_media_id_empty() {
        let response = json!({"id": ""});
        let result = validate_media_id("", &response);
        assert!(result.is_err());
        match result.unwrap_err() {
            FbapiError::InvalidMediaId { id, .. } => assert_eq!(id, ""),
            _ => panic!("Expected InvalidMediaId error"),
        }
    }

    #[test]
    fn test_validate_media_id_non_numeric() {
        let response = json!({"id": "abc123"});
        let result = validate_media_id("abc123", &response);
        assert!(result.is_err());
        match result.unwrap_err() {
            FbapiError::InvalidMediaId { id, .. } => assert_eq!(id, "abc123"),
            _ => panic!("Expected InvalidMediaId error"),
        }
    }

    #[test]
    fn test_validate_media_id_negative() {
        let response = json!({"id": "-123"});
        let result = validate_media_id("-123", &response);
        assert!(result.is_err());
        match result.unwrap_err() {
            FbapiError::InvalidMediaId { id, .. } => assert_eq!(id, "-123"),
            _ => panic!("Expected InvalidMediaId error"),
        }
    }
}
