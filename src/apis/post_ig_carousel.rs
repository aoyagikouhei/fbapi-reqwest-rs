use crate::apis::check_ig_media::check_ig_media_loop;
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

        check_ig_media_loop(
            &self.make_path(&format!(
                "{}?fields=status,status_code&access_token={}",
                creation_id, access_token
            )),
            check_retry_count,
            check_video_delay,
            retry_count,
            &self.client,
            &log,
        )
        .await?;

        self.post_ig_media_publish(
            &access_token,
            &account_igid,
            &creation_id,
            retry_count,
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

/*
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
 */