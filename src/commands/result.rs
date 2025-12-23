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
    List(ListResult),
    Show(Box<ShowResult>),
    Delete(DeleteResult),
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

/// リストコマンドの結果
#[derive(Debug, Clone, Serialize)]
pub struct ListResult {
    /// 動画リスト（人間向け簡略版）
    pub videos: Vec<VideoInfo>,
    /// 合計数
    pub total_count: usize,
    /// 完全なAPIレスポンスデータ（機械向け、--machineフラグ時のみ）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_assets: Option<Vec<crate::api::types::AssetData>>,
}

/// アセット詳細表示コマンドの結果
#[derive(Debug, Clone, Serialize)]
pub struct ShowResult {
    /// アセットID
    pub asset_id: String,
    /// ステータス (preparing, ready, errored)
    pub status: String,
    /// 動画時間（秒）
    pub duration: Option<f64>,
    /// アスペクト比
    pub aspect_ratio: Option<String>,
    /// ビデオ品質
    pub video_quality: Option<String>,
    /// 作成日時（Unix timestamp）
    pub created_at: String,
    /// 再生ID
    pub playback_ids: Vec<crate::api::types::PlaybackId>,
    /// HLS再生URL
    pub hls_url: Option<String>,
    /// MP4再生URL
    pub mp4_url: Option<String>,
    /// 動画トラック情報
    pub tracks: Option<Vec<crate::api::types::Track>>,
    /// Static Renditions（MP4など）
    pub static_renditions: Option<crate::api::types::StaticRenditionsWrapper>,
    /// 完全なAPIレスポンスデータ（機械向け、--machineフラグ時のみ）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_asset: Option<crate::api::types::AssetData>,
}

/// 削除コマンドの結果
#[derive(Debug, Clone, Serialize)]
pub struct DeleteResult {
    /// 削除されたアセットID
    pub asset_id: String,
}

/// 動画情報
#[derive(Debug, Clone, Serialize)]
pub struct VideoInfo {
    /// アセットID
    pub asset_id: String,
    /// ステータス (preparing, ready, errored)
    pub status: String,
    /// 再生ID
    pub playback_id: Option<String>,
    /// HLS再生URL
    pub hls_url: Option<String>,
    /// MP4再生URL
    pub mp4_url: Option<String>,
    /// 動画時間（秒）
    pub duration: Option<f64>,
    /// 作成日時（Unix timestamp）
    pub created_at: String,
    /// アスペクト比
    pub aspect_ratio: Option<String>,
}
