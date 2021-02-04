use crate::*;

impl Fbapi {
    pub async fn get_object(
        &self,
        access_token: &str,
        app_secret: Option<&str>,
        fbid: &str,
        fields: &str,
        params: &[(&str, &str)],
        retry_count: usize,
        log:  impl Fn(LogParams),
    ) -> Result<serde_json::Value, FbapiError> {
        let mut query = [("access_token", access_token), ("fields", fields)]
            .iter()
            .chain(params)
            .map(|&(key, value)| format!("{}={}", key, value))
            .collect::<Vec<_>>()
            .join("&");

        if let Some(secret) = app_secret {
            let appsecret_proof = sign(access_token, secret);
            query += &format!("&{}={}", "appsecret_proof", appsecret_proof);
        }

        let path = self.make_path(&format!("{}?{}", fbid, query));
        let params = LogParams::new(&path, &vec![]);
        if self.rate_limit_emulation {
            (log)(params);
            return Err(FbapiError::Facebook((*ERROR_VALUE).clone()));
        }
        let json = self
            .execute_retry(
                retry_count,
                || async { self.client.get(&path).send().await.map_err(|e| e.into()) },
                &log,
                params,
            )
            .await?;

        Ok(json)
    }
}