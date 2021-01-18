pub mod error;

#[macro_use]
extern crate serde_json;

use async_trait::async_trait;
use crate::error::FbapiError;
use crypto::mac::Mac;
use once_cell::sync::Lazy;
use std::{future::Future, time::Duration};

const GRAPH_PREFIX: &'static str = "https://graph.facebook.com/";
//const VIDEO_PREFIX: &'static str = "https://graph-video.facebook.com/";

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

#[async_trait]
pub trait FbapiLog {
    async fn log(&self, params: LogParams) {
        match params.result {
            Some(ref result) => println!("params {},{:?},{},{:?}",  params.path, params.params, params.count, result),
            None => println!("params {},{:?},{}",  params.path, params.params, params.count),
        }
    }
}

pub struct Fbapi
{
    client: reqwest::Client,
    version: String,
    log_executor: Box<dyn FbapiLog + 'static + Sync>,
    rate_limit_emulation: bool,
}

impl Fbapi
{
    pub fn new(
        version: &str,
        timeout_seconds: u64,
        log_executor: Box<dyn FbapiLog + 'static + Sync>,
        rate_limit_emulation: bool,
    ) -> Result<Self, FbapiError> {
        Ok(Self {
            client: reqwest::ClientBuilder::new()
                .timeout(Duration::from_secs(timeout_seconds))
                .build()?,
            version: version.to_owned(),
            log_executor: log_executor,
            rate_limit_emulation: rate_limit_emulation,
        })
    }

    pub async fn get_object(
        &self,
        access_token: &str,
        app_secret: Option<&str>,
        fbid: &str,
        fields: &str,
        params: &[(&str, &str)],
        retry_count: usize,
    ) -> Result<serde_json::Value, FbapiError> {
        let mut query = [("access_token", access_token), ("fields", fields)]
            .iter()
            .chain(params)
            .map(|&(key, value)| format!("{}={}", key, value))
            .collect::<Vec<_>>()
            .join("&");

        if let Some(secret) = app_secret {
            let appsecret_proof = sign(access_token, secret);
            query += &format!("&{}={}", "appsecret_proof", appsecret_proof);
        }

        let path = format!("{}{}/{}?{}", GRAPH_PREFIX, self.version, fbid, query);
        let params = LogParams {
            path: path.clone(),
            params: vec![],
            count: 0,
            result: None,
        };
        if self.rate_limit_emulation {
            self.log_executor.log(params).await;
            return Err(FbapiError::Facebook((*ERROR_VALUE).clone()));
        }
        let json = self
            .execute_retry(
                retry_count,
                || async { self.client.get(&path).send().await.map_err(|e| e.into()) },
                &self.log_executor,
                params,
            )
            .await?;

        Ok(json)
    }

    async fn execute_retry<Executor, ResponseFutuer>(
        &self,
        retry_count: usize,
        executor: Executor,
        log_executor: &Box<dyn FbapiLog + 'static + Sync>,
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
            log_executor.log(params).await;
            match executor().await {
                Ok(response) => match response.json::<serde_json::Value>().await {
                    Ok(json) => {
                        if json["error"].is_object() {
                            return Err(FbapiError::Facebook(json));
                        } else {
                            let mut params = src_params.clone();
                            params.count = count;
                            params.result = Some(json.clone());
                            log_executor.log(params).await;
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

#[cfg(test)]
mod tests {
    use super::*;

    struct Dummy;
    impl FbapiLog for Dummy {}

    #[tokio::test]
    async fn it_works() {
        let api = Fbapi::new(
            "v8.0",
            10,
            Box::new(Dummy{}),
            true,
        )
        .unwrap();
        let res = api.get_object("xxxx", None, "aaa", "", &vec![], 2).await;
        println!("{:?}", res);
    }
}
