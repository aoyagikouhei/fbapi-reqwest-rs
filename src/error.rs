use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FbapiError {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error("facebook error: {0}")]
    Facebook(serde_json::Value),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error("Facebook unexpected json: {0}")]
    UnExpected(serde_json::Value),

    #[error("Facebook viedo error")]
    VideoError,

    #[error("Facebook viedo check loop timeout error")]
    VideoTimeout,

    #[error("Instagram viedo error: {0}")]
    IgVideoError(serde_json::Value),

    #[error("Copyright violation detected")]
    CopyRight,

    #[error("Facebook video delayed, retry recommended")]
    VideoDelayed,
}

// ユーザに表示するエラー内容
const SHOULD_REOAUTH: &'static str =
    "アカウントの認証エラーで投稿が失敗しました。アカウントを再認証してください。";

// Graph API から返ってくるエラー
#[derive(Deserialize, Debug)]
struct FbError {
    error: FbDetailError,
}

#[derive(Deserialize, Debug)]
struct FbDetailError {
    code: u64,
    error_subcode: Option<u64>,
}

impl FbapiError {
    pub fn make_error_content_for_user(&self) -> String {
        match self {
            FbapiError::Facebook(value) => {
                // FbError に変換
                serde_json::from_value(value.clone())
                    .map(|fb_error: FbError| match fb_error.error {
                        // アクセストークン有効期限切れのエラーコードのとき
                        FbDetailError { code: 190, .. } => SHOULD_REOAUTH.to_owned(),

                        // 認証エラーのサブコードのとき
                        // https://developers.facebook.com/docs/graph-api/using-graph-api/error-handling
                        FbDetailError {
                            error_subcode: Some(error_subcode),
                            ..
                        } if [458, 459, 460, 463, 464, 467, 492].contains(&error_subcode) => {
                            SHOULD_REOAUTH.to_owned()
                        }

                        // その他のエラーのときはユーザに表示しない
                        _ => "".to_owned(),
                    })
                    .unwrap_or("".to_owned())
            }

            // Graph API のエラーでない場合はユーザに表示しない
            _ => "".to_owned(),
        }
    }
}
