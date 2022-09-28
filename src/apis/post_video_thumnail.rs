use crate::*;
use reqwest::multipart::Form;

impl Fbapi {
    pub async fn post_video_thumnail(
        &self,
        access_token: &str,
        video_id: &str,
        bytes: rusoto_core::ByteStream,
        log: impl Fn(LogParams),
    ) -> Result<serde_json::Value, FbapiError> {
        let path = self.make_path(&format!("{}/thumbnails", video_id));
        let params = vec![
            ("access_token", access_token),
            ("video_id", video_id),
        ];
        let log_params = LogParams::new(&path, &params);
        let part = make_part("thumnail", bytes)?;
        let form = Form::new()
            .text("access_token", access_token.to_string())
            .text("is_preferred", "true")
            .part("source", part);
        execute_form(&self.client, &path, form, &log, log_params).await
    }
}
