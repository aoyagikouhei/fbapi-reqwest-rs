use crate::*;

impl Fbapi {
    pub async fn create_album(
        &self,
        access_token: &str,
        page_fbid: &str,
        name: &str,
        message: &str,
        retry_count: usize,
        log:  impl Fn(LogParams),
    ) -> Result<serde_json::Value, FbapiError> {
        let path = self.make_path(&format!("{}/albums", page_fbid));
        let params = vec![("access_token", access_token), ("name", name), ("message", message)];
        let log_params = LogParams::new(&path, &params);
        if self.rate_limit_emulation {
            (log)(log_params);
            return Err(FbapiError::Facebook((*ERROR_VALUE).clone()));
        }

        let json = self
            .execute_retry(
                retry_count,
                || async { self.client.post(&path).form(&params).send().await.map_err(|e| e.into()) },
                &log,
                log_params,
            )
            .await?;
        Ok(json)
    }
}