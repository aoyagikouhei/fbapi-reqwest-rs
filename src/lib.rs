pub mod apis;
pub mod batch_request;
pub mod error;

#[macro_use]
extern crate serde_json;

use crate::error::FbapiError;
use crypto::mac::Mac;
use once_cell::sync::Lazy;
use reqwest::{multipart::Part, Body};
use std::{future::Future, path::Path, time::Duration};
use tokio::{fs::File, time::delay_for};
use tokio_util::codec::{BytesCodec, FramedRead};

const GRAPH_PREFIX: &'static str = "https://graph.facebook.com/";
const VIDEO_PREFIX: &'static str = "https://graph-video.facebook.com/";

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
            Ok(response) => match response.json::<serde_json::Value>().await {
                Ok(json) => {
                    if json["error"].is_object() {
                        return Err(FbapiError::Facebook(json));
                    } else {
                        let mut params = src_params.clone();
                        params.count = count;
                        params.result = Some(json.clone());
                        log(params);
                        return Ok(json);
                    }
                }
                Err(err) => last_error = err.into(),
            },
            Err(err) => last_error = err.into(),
        };
        count = count + 1;
        if count >= retry_count {
            break;
        }
    }
    Err(last_error)
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
        }
    }
}

pub(crate) async fn make_part(path: &Path) -> Result<Part, FbapiError> {
    let file_name: String = path.file_name().unwrap().to_string_lossy().into();
    let opeded_file = File::open(path).await?;
    let reader = FramedRead::new(opeded_file, BytesCodec::new());
    Part::stream(Body::wrap_stream(reader))
        .file_name(file_name.clone())
        .mime_str("application/octet-stream")
        .map_err(|e| e.into())
}

pub(crate) async fn sleep(seconds: usize) {
    delay_for(Duration::from_secs(seconds as u64)).await
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
}
