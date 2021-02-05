use crate::*;
use reqwest::multipart::Form;
use tempfile::NamedTempFile;

impl Fbapi {
    pub async fn post_album_photo(
        &self,
        access_token: &str,
        album_fbid: &str,
        file: &NamedTempFile,
        file_path: &str,
        message: &str,
        log: impl Fn(LogParams),
    ) -> Result<serde_json::Value, FbapiError> {
        let path = self.make_path(&format!("{}/photos", album_fbid));
        let params = vec![
            ("access_token", access_token),
            ("file_path", file_path),
            ("message", message),
            ("published", "true"),
        ];
        let log_params = LogParams::new(&path, &params);
        execute_retry(
            0,
            || async {
                let form = Form::new()
                    .text("access_token", access_token.to_string())
                    .text("message", message.to_string())
                    .text("published", "true")
                    .part("source", make_part(file.path()).await?);
                self.client
                    .post(&path)
                    .multipart(form)
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
