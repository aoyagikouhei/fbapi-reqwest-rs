use crate::*;

impl Fbapi {
    pub async fn post_feed_array(
        &self,
        page_fbid: &str,
        params: &[(&str, &str)],
        log: impl Fn(LogParams),
    ) -> Result<serde_json::Value, FbapiError> {
        let path = self.make_path(&format!("{}/feed", page_fbid));
        let log_params = LogParams::new(&path, &vec![]);
        execute_retry(
            0,
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
