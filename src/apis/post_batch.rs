use crate::*;

impl Fbapi {
    pub async fn post_batch(
        &self,
        access_token: &str,
        app_secret: Option<&str>,
        batch: batch_request::BatchRequest,
        retry_count: usize,
        log:  impl Fn(LogParams),
    ) -> Result<Vec<Result<serde_json::Value, FbapiError>>, FbapiError> {
        let batch_string = batch.to_string();
        let mut query = vec![
                ("access_token", access_token),
                ("include_headers", "false"),
                ("batch", batch_string.as_str()),
            ];

        let appsecret_proof = app_secret.map(|secret| sign(access_token, secret)).unwrap_or("".to_string());
        if app_secret.is_some() {
            query.push(("appsecret_proof", &appsecret_proof));
        }

        let path = self.make_path("");
        let params = LogParams::new(&path, &query);

        if self.rate_limit_emulation {
            (log)(params);
            return batch_request::response_shaper(generate_rate_limit_array_for_batch(batch.batch_count));
        }

        let json = self
            .execute_retry(
                retry_count,
                || async { self.client.post(&path).form(&query).send().await.map_err(|e| e.into()) },
                &log,
                params,
            )
            .await?;
        crate::batch_request::response_shaper(json)
    }
}

fn generate_rate_limit_array_for_batch(count: usize) -> serde_json::Value {
    let item = json!({
        "code": 400,
        "headers": [],
        "body": ERROR_VALUE.to_string(),
    });
    let mut vec = Vec::new();
    vec.resize(count, item);
    serde_json::Value::Array(vec)
}