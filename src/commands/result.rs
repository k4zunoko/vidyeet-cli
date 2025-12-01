/// コマンド実行結果を表す型
///
/// 各コマンドはこの型を返し、プレゼンテーション層（main.rs/cli.rs）で
/// 人間向けと機械向けの出力フォーマットを決定する。
use serde::Serialize;

/// コマンド実行結果の統一型
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum CommandResult {
    Login(LoginResult),
    Logout(LogoutResult),
    Upload(UploadResult),
    Status(StatusResult),
    Help,
}

/// ログインコマンドの結果
#[derive(Debug, Clone, Serialize)]
pub struct LoginResult {
    /// 既にログイン済みだったか（上書き更新の場合true）
    pub was_logged_in: bool,
}

/// ログアウトコマンドの結果
#[derive(Debug, Clone, Serialize)]
pub struct LogoutResult {
    /// ログイン状態だったか
    pub was_logged_in: bool,
}

/// ステータスコマンドの結果
#[derive(Debug, Clone, Serialize)]
pub struct StatusResult {
    /// 認証が通っているか
    pub is_authenticated: bool,
    /// マスキングされたToken ID（認証情報がある場合）
    pub token_id: Option<String>,
}

/// アップロードコマンドの結果
#[derive(Debug, Clone, Serialize)]
pub struct UploadResult {
    /// アセットID
    pub asset_id: String,
    /// 再生ID（HLS/MP4のURL構築に使用）
    pub playback_id: Option<String>,
    /// HLS再生URL（すぐに利用可能）
    pub hls_url: Option<String>,
    /// MP4再生URL（生成完了時のみ）
    pub mp4_url: Option<String>,
    /// MP4のステータス（ready, generating）
    pub mp4_status: Mp4Status,
    /// ファイルパス
    pub file_path: String,
    /// ファイルサイズ（bytes）
    pub file_size: u64,
    /// ファイル形式（拡張子）
    pub file_format: String,
    /// 削除した古い動画の数
    pub deleted_old_videos: usize,
}

/// MP4の生成ステータス
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mp4Status {
    /// すぐに利用可能
    Ready,
    /// バックグラウンドで生成中
    Generating,
}

impl CommandResult {
    /// 成功メッセージを取得（人間向け出力用）
    pub fn success_message(&self) -> String {
        match self {
            CommandResult::Login(r) => {
                if r.was_logged_in {
                    "Login credentials updated!".to_string()
                } else {
                    "Login successful!".to_string()
                }
            }
            CommandResult::Logout(r) => {
                if r.was_logged_in {
                    "Logged out successfully.".to_string()
                } else {
                    "Already logged out.".to_string()
                }
            }
            CommandResult::Upload(_) => "Upload completed successfully!".to_string(),
            CommandResult::Status(r) => {
                if r.is_authenticated {
                    "Authenticated".to_string()
                } else {
                    "Not authenticated".to_string()
                }
            }
            CommandResult::Help => "".to_string(),
        }
    }
}
