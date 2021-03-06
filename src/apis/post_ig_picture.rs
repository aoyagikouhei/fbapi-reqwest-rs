use crate::*;

impl Fbapi {
    pub async fn post_ig_picture(
        &self,
        access_token: &str,
        account_igid: &str,
        image_url: &str,
        caption: &str,
        retry_count: usize,
        log: impl Fn(LogParams),
    ) -> Result<serde_json::Value, FbapiError> {
        let creation_id = post(
            &self.make_path(&format!("{}/media", account_igid)),
            &access_token,
            &image_url,
            &caption,
            retry_count,
            &self.client,
            &log,
        )
        .await?;

        publish(
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
    image_url: &str,
    caption: &str,
    retry_count: usize,
    client: &reqwest::Client,
    log: impl Fn(LogParams),
) -> Result<String, FbapiError> {
    let params = vec![
        ("access_token", access_token),
        ("image_url", image_url),
        ("caption", caption),
    ];
    let log_params = LogParams::new(&path, &params);
    let res = execute_retry(
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
    .await?;
    match res["id"].as_str() {
        Some(s) => Ok(s.to_owned()),
        None => return Err(FbapiError::UnExpected(res)),
    }
}

async fn publish(
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
