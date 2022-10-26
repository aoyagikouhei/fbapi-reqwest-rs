use crate::*;

async fn check_ig_media(
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

pub(crate) async fn check_ig_media_loop(
    path: &str,
    check_retry_count: usize,
    check_video_delay: usize,
    retry_count: usize,
    client: &reqwest::Client,
    log: &impl Fn(LogParams),
) -> Result<(), FbapiError> {
    for _ in 0..check_retry_count {
        match check_ig_media(path, retry_count, client, log)
            .await?
            .as_str()
        {
            "FINISHED" => return Ok(()),
            "IN_PROGRESS" => {}
            _ => return Err(FbapiError::VideoError),
        }
        sleep(check_video_delay).await;
    }
    Err(FbapiError::VideoTimeout)
}
