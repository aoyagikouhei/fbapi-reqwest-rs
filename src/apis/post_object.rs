use crate::*;

impl Fbapi {
    pub async fn post_object(
        &self,
        access_token: &str,
        fbid: &str,
        params: &[(&str, &str)],
        retry_count: usize,
        log: impl Fn(LogParams),
    ) -> Result<serde_json::Value, FbapiError> {
        let path = self.make_path(fbid);
        let params: Vec<(&str, &str)> = [("access_token", access_token)]
            .iter()
            .chain(params)
            .map(|&x| x)
            .collect();
        let log_params = LogParams::new(&path, &params);
        execute_retry(
            retry_count,
            || async {
                self.client
                    .post(&path)
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
}
