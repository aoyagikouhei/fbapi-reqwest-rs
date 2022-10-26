use crate::*;

impl Fbapi {
    pub async fn post_ig_carousel(
        &self,
        access_token: &str,
        account_igid: &str,
        caption: &str,
        children: &Vec<String>,
        check_retry_count: usize,
        check_video_delay: usize,
        retry_count: usize,
        log: impl Fn(LogParams),
    ) -> Result<serde_json::Value, FbapiError> {
        let creation_id = post(
            &self.make_path(&format!("{}/media", account_igid)),
            &access_token,
            &caption,
            &children,
            retry_count,
            &self.client,
            &log,
        )
        .await?;

        check_loop(
            &self.make_path(&format!(
                "{}?fields=status_code&access_token={}",
                creation_id, access_token
            )),
            check_retry_count,
            check_video_delay,
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
    caption: &str,
    children: &Vec<String>,
    retry_count: usize,
    client: &reqwest::Client,
    log: impl Fn(LogParams),
) -> Result<String, FbapiError> {
    let children_str = &children.join(",");
    let params = vec![
        ("access_token", access_token),
        ("media_type", "CAROUSEL"),
        ("children", children_str),
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

async fn check(
    path: &str,
    retry_count: usize,
    client: &reqwest::Client,
    log: &impl Fn(LogParams),
) -> Result<String, FbapiError> {
    let log_params = LogParams::new(&path, &vec![]);
    let res = execute_retry(
        retry_count,
        || async { client.get(path).send().await.map_err(|e| e.into()) },
        log,
        log_params,
    )
    .await?;
    match res["status_code"].as_str() {
        Some(s) => Ok(s.to_owned()),
        None => return Err(FbapiError::UnExpected(res)),
    }
}

async fn check_loop(
    path: &str,
    check_retry_count: usize,
    check_video_delay: usize,
    retry_count: usize,
    client: &reqwest::Client,
    log: &impl Fn(LogParams),
) -> Result<(), FbapiError> {
    for _ in 0..check_retry_count {
        match check(path, retry_count, client, log).await?.as_str() {
            "FINISHED" => return Ok(()),
            "IN_PROGRESS" => {}
            _ => return Err(FbapiError::VideoError),
        }
        sleep(check_video_delay).await;
    }
    Err(FbapiError::VideoTimeout)
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
