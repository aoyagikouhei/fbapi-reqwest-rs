use crate::apis::check_ig_media::check_ig_media_loop;
use crate::*;

impl Fbapi {
    pub async fn post_ig_reel(
        &self,
        access_token: &str,
        account_igid: &str,
        video_url: &str,
        cover_url: Option<&str>,
        caption: &str,
        is_share_to_feed: bool,
        check_retry_count: usize,
        check_video_delay: usize,
        retry_count: usize,
        log: impl Fn(LogParams),
    ) -> Result<serde_json::Value, FbapiError> {
        let creation_id = post(
            &self.make_path(&format!("{}/media", account_igid)),
            &access_token,
            &video_url,
            cover_url,
            &caption,
            is_share_to_feed,
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
    video_url: &str,
    cover_url: Option<&str>,
    caption: &str,
    is_share_to_feed: bool,
    retry_count: usize,
    client: &reqwest::Client,
    log: impl Fn(LogParams),
) -> Result<String, FbapiError> {
    let mut params = vec![
        ("access_token", access_token),
        ("media_type", "REELS"),
        ("video_url", video_url),
        ("caption", caption),
        (
            "share_to_feed",
            if is_share_to_feed { "true" } else { "false" },
        ),
    ];

    if let Some(url) = cover_url {
        params.push(("cover_url", url));
    }

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
