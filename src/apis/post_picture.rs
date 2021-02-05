use crate::*;
use reqwest::multipart::Form;
use tempfile::NamedTempFile;

impl Fbapi {
    pub async fn post_picture(
        &self,
        access_token: &str,
        page_fbid: &str,
        file: &NamedTempFile,
        file_path: &str,
        caption: &str,
        log: impl Fn(LogParams),
    ) -> Result<serde_json::Value, FbapiError> {
        let path = self.make_path(&format!("{}/photos", page_fbid));
        let params = vec![
            ("access_token", access_token),
            ("file_path", file_path),
            ("caption", caption),
            ("published", "false"),
        ];
        let log_params = LogParams::new(&path, &params);
        execute_retry(
            0,
            || async {
                let form = Form::new()
                    .text("access_token", access_token.to_string())
                    .text("caption", caption.to_string())
                    .text("published", "false")
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
