use crate::*;
use reqwest::multipart::Form;

impl Fbapi {
    pub async fn post_picture(
        &self,
        access_token: &str,
        page_fbid: &str,
        media_file: &std::fs::File,
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
        let part = make_part(file_path, media_file)?;
        let form = Form::new()
            .text("access_token", access_token.to_string())
            .text("caption", caption.to_string())
            .text("published", "false")
            .part("source", part);
        execute_form(&self.client, &path, form, &log, log_params).await
    }
}
