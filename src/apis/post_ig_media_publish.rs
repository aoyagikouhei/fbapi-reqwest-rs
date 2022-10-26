use crate::*;

impl Fbapi {
    pub async fn post_ig_media_publish(
        &self,
        access_token: &str,
        account_igid: &str,
        creation_id: &str,
        retry_count: usize,
        log: impl Fn(LogParams),
    ) -> Result<serde_json::Value, FbapiError> {
        post(
            &self.make_path(&format!("{}/media_publish", account_igid)),
            &access_token,
            &creation_id,
            retry_count,
            &self.client,
            &log,
        )
        .await
    }
}

async fn post(
    path: &str,
    access_token: &str,
    creation_id: &str,
    retry_count: usize,
    client: &reqwest::Client,
    log: impl Fn(LogParams),
) -> Result<serde_json::Value, FbapiError> {
    let params = vec![("access_token", access_token), ("creation_id", creation_id)];
    let log_params = LogParams::new(&path, &params);
    execute_retry(
        retry_count,
        || async {
            client
                .post(path)
                .form(&params)
                .send()
                .await
                .map_err(|e| e.into())
        },
        &log,
        log_params,
    )
    .await
}
