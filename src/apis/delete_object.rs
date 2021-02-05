use crate::*;

impl Fbapi {
    pub async fn delete_object(
        &self,
        access_token: &str,
        fbid: &str,
        fields: &str,
        params: &[(&str, &str)],
        retry_count: usize,
        log: impl Fn(LogParams),
    ) -> Result<serde_json::Value, FbapiError> {
        let query = [("access_token", access_token), ("fields", fields)]
            .iter()
            .chain(params)
            .map(|&(key, value)| format!("{}={}", key, value))
            .collect::<Vec<_>>()
            .join("&");
        let path = self.make_path(&format!("{}?{}", fbid, query));
        let log_params = LogParams::new(&path, &vec![]);
        execute_retry(
            retry_count,
            || async { self.client.delete(&path).send().await.map_err(|e| e.into()) },
            &log,
            log_params,
        )
        .await
    }
}
