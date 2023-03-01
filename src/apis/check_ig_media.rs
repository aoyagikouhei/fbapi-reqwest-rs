use crate::*;

async fn check_ig_media(
    path: &str,
    retry_count: usize,
    client: &reqwest::Client,
    log: &impl Fn(LogParams),
) -> Result<serde_json::Value, FbapiError> {
    let log_params = LogParams::new(&path, &vec![]);
    let res = execute_retry(
        retry_count,
        || async { client.get(path).send().await.map_err(|e| e.into()) },
        log,
        log_params,
    )
    .await?;
    Ok(res)
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
        let res = check_ig_media(path, retry_count, client, log).await?;
        let status_code = match res["status_code"].as_str() {
            Some(s) => s.to_owned(),
            None => return Err(FbapiError::UnExpected(res)),
        };
        match status_code.as_str() {
            "FINISHED" => return Ok(()),
            "IN_PROGRESS" => {}
            _ => return Err(FbapiError::IgVideoError(res)),
        }
        sleep_sec(check_video_delay).await;
    }
    Err(FbapiError::VideoTimeout)
}
