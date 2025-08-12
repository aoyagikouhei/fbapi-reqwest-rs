use serde_json::Value;

use crate::*;

impl Fbapi {
    pub async fn post_video_reel(
    &self,
    access_token: &str,
    page_fbid: &str,
    file_url: &str,
    description: &str,
    thumb: Option<rusoto_core::ByteStream>,
    long_client: reqwest::Client,
    log: impl Fn(LogParams),
    ) -> Result<Value, FbapiError> {

        // １．アップロード用のURLを取得して動画をアップロードする。
        let path = self.make_path(&format!("{}/video_reels", page_fbid));
        let params = vec![("access_token", access_token), ("upload_phase", "start")];
        let log_params = LogParams::new(&path, &params);
        let res_request: serde_json::Value = execute_retry(
            0,
            || async {
                long_client
                    .post(&path)
                    .form(&params)
                    .send()
                    .await
                    .map_err(|e| e.into())
            },
            &log,
            log_params,
        )
        .await?;

        let upload_reel_url = res_request["upload_url"].as_str();
        let video_id = res_request["video_id"].as_str();

        // ２．video_urlを使って動画をアップロードする。
        if let (Some(upload_reel_url), Some(video_id)) = (upload_reel_url, video_id) {
            let log_params = LogParams::new(upload_reel_url, &vec![("file_url", file_url)]);
            let upload_response: serde_json::Value = execute_retry(
                0,
                || async {
                    long_client
                        .post(upload_reel_url)
                        .header("Authorization", format!("OAuth {}", access_token))
                        .header("file_url", file_url)
                        .send()
                        .await
                        .map_err(|e| e.into())
                },
                &log,
                log_params,
            )
            .await?;

            if upload_response.get("success").and_then(|v| v.as_bool()) != Some(true) {
                return Err(FbapiError::UnExpected(upload_response.clone()));
            }

            let check_path = self.make_path(&format!(
                "{}?fields=status&access_token={}",
                video_id, access_token
            ));

            // ３．ステップ２でアップロードした動画のステータスを確認する。
            loop {
                let log_params = LogParams::new(&check_path, &vec![]);
                let status_res: serde_json::Value = execute_retry(
                    0,
                    || async {
                        self.client
                            .get(&check_path)
                            .send()
                            .await
                            .map_err(|e| e.into())
                    },
                    &log,
                    log_params,
                )
                .await?;

                let uploading_status = status_res["status"]["uploading_phase"]["status"].as_str();

                match uploading_status {
                    Some("complete") => break,
                    Some("in_progress") => {
                        sleep_sec(1).await;
                        continue;
                    }
                    Some("error") | Some("failed") => {
                        return Err(FbapiError::UnExpected(json!({
                            "error": "uploading_phase",
                            "status": status_res
                        })));
                    }
                    _ => {
                        if let Some(error_info) =
                            status_res["status"]["uploading_phase"].get("error")
                        {
                            return Err(FbapiError::UnExpected(json!({
                                "error": "uploading_phase",
                                "status": status_res
                            })));
                        }
                        return Err(FbapiError::UnExpected(json!({
                            "error": "uploading_phase",
                            "status": status_res
                        })));
                    }
                }
            }

            // ４．ステップ２でアップロードした動画の著作権を確認する。
            loop {
                let log_params = LogParams::new(&check_path, &vec![]);
                let status_res: serde_json::Value = execute_retry(
                    0,
                    || async {
                        self.client
                            .get(&check_path)
                            .send()
                            .await
                            .map_err(|e| e.into())
                    },
                    &log,
                    log_params,
                )
                .await?;

                let copyright_status =
                    status_res["status"]["copyright_check_status"]["status"].as_str();

                match copyright_status {
                    Some("complete") => {
                        let matches_found = status_res["status"]["copyright_check_status"]
                            ["matches_found"]
                            .as_bool();
                        if matches_found == Some(true) {
                            return Err(FbapiError::CopyRight);
                        }
                        break;
                    }
                    Some("in_progress") => {
                        sleep_sec(2).await;
                        continue;
                    }
                    Some("error") | Some("failed") => {
                        return Err(FbapiError::UnExpected(json!({
                            "error": "copyright_check_status",
                            "status": status_res
                        })));
                    }
                    _ => {
                        return Err(FbapiError::UnExpected(json!({
                            "error": "copyright_check_status",
                            "status": status_res
                        })))
                    }
                }
            }

            // ５．サムネイルがある場合はアップロードする。
            match thumb {
                Some(bytes) => {
                    self.post_video_thumnail(access_token, &video_id, bytes, &log)
                        .await?;
                }
                None => {}
            };

            // ６．動画リールを公開する。
            let finish_path = self.make_path(&format!("{}/video_reels", page_fbid));
            let finish_params = vec![
                ("access_token", access_token),
                ("video_id", video_id),
                ("upload_phase", "finish"),
                ("video_state", "PUBLISHED"),
                ("description", description),
            ];
            let log_params = LogParams::new(&finish_path, &finish_params);
            let finish_res: serde_json::Value = execute_retry(
                0,
                || async {
                    long_client
                        .post(&finish_path)
                        .form(&finish_params)
                        .send()
                        .await
                        .map_err(|e| e.into())
                },
                &log,
                log_params,
            )
            .await?;

            if finish_res["success"].as_bool() != Some(true) {
                return Err(FbapiError::UnExpected(finish_res));
            }

            // post_id が返却されることを確認する。
            let post_id = finish_res["post_id"].as_str();
            match post_id {
                Some(id) => {
                    // ７．processing_phase を確認する。
                    loop {
                        let log_params = LogParams::new(&check_path, &vec![]);
                        let status_res: serde_json::Value = execute_retry(
                            0,
                            || async {
                                self.client
                                    .get(&check_path)
                                    .send()
                                    .await
                                    .map_err(|e| e.into())
                            },
                            &log,
                            log_params,
                        )
                        .await?;

                        let processing_status =
                            status_res["status"]["processing_phase"]["status"].as_str();

                        match processing_status {
                            Some("complete") => break,
                            Some("in_progress") | Some("not_started") => {
                                if status_res["status"]["processing_phase"]["error"].is_object() {
                                    return Err(FbapiError::UnExpected(json!({
                                        "error": "processing_phase",
                                        "status": status_res
                                    })));
                                }
                                sleep_sec(2).await;
                                continue;
                            }
                            Some("error") | Some("failed") => {
                                return Err(FbapiError::UnExpected(json!({
                                    "error": "processing_phase",
                                    "status": status_res
                                })));
                            }
                            _ => {
                                return Err(FbapiError::UnExpected(json!({
                                    "error": "processing_phase",
                                    "status": status_res
                                })));
                            }
                        }
                    }

                    // ８．publishing_phase を確認する。
                    loop {
                        let log_params = LogParams::new(&check_path, &vec![]);
                        let status_res: serde_json::Value = execute_retry(
                            0,
                            || async {
                                self.client
                                    .get(&check_path)
                                    .send()
                                    .await
                                    .map_err(|e| e.into())
                            },
                            &log,
                            log_params,
                        )
                        .await?;

                        let publishing_status =
                            status_res["status"]["publishing_phase"]["status"].as_str();

                        match publishing_status {
                            Some("complete") => break,
                            Some("in_progress") | Some("not_started") => {
                                sleep_sec(2).await;
                                continue;
                            }
                            Some("error") | Some("failed") => {
                                return Err(FbapiError::UnExpected(json!({
                                    "error": "publishing_phase",
                                    "status": status_res
                                })));
                            }
                            _ => {
                                return Err(FbapiError::UnExpected(json!({
                                    "error": "publishing_phase",
                                    "status": status_res
                                })));
                            }
                        }
                    }
                    return Ok(finish_res);
                }
                None => return Err(FbapiError::UnExpected(finish_res)),
            }
        } else {
            return Err(FbapiError::UnExpected(res_request));
        }
    }
}
