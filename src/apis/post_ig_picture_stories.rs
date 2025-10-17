use crate::apis::check_ig_media::check_ig_media_loop;
use crate::*;

impl Fbapi {
    pub async fn post_ig_picture_stories(
        &self,
        access_token: &str,
        account_igid: &str,
        image_url: &str,
        check_retry_count: usize,
        check_delay: usize,
        retry_count: usize,
        log: impl Fn(LogParams),
    ) -> Result<serde_json::Value, FbapiError> {
        let creation_id = self
            .upload_ig_picture_stories(
                &access_token,
                &account_igid,
                &image_url,
                check_retry_count,
                check_delay,
                retry_count,
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

    /// Upload a picture stories and poll its status, returning the creation_id without publishing.
    /// This allows you to handle the publish step separately using `post_ig_media_publish`.
    pub async fn upload_ig_picture_stories(
        &self,
        access_token: &str,
        account_igid: &str,
        image_url: &str,
        check_retry_count: usize,
        check_delay: usize,
        retry_count: usize,
        log: impl Fn(LogParams),
    ) -> Result<String, FbapiError> {
        let creation_id = post(
            &self.make_path(&format!("{}/media", account_igid)),
            &access_token,
            &image_url,
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
            check_delay,
            retry_count,
            &self.client,
            &log,
        )
        .await?;

        Ok(creation_id)
    }
}

async fn post(
    path: &str,
    access_token: &str,
    image_url: &str,
    retry_count: usize,
    client: &reqwest::Client,
    log: impl Fn(LogParams),
) -> Result<String, FbapiError> {
    let params = vec![
        ("access_token", access_token),
        ("media_type", "STORIES"),
        ("image_url", image_url),
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
