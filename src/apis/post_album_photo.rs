use crate::*;
use reqwest::multipart::Form;

impl Fbapi {
    pub async fn post_album_photo(
        &self,
        access_token: &str,
        album_fbid: &str,
        file_path: &str,
        message: &str,
        bytes: rusoto_core::ByteStream,
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
        let part = make_part(file_path, bytes)?;
        let form = Form::new()
            .text("access_token", access_token.to_string())
            .text("message", message.to_string())
            .text("published", "true")
            .part("source", part);
        execute_form(&self.client, &path, form, &log, log_params).await
    }
}
