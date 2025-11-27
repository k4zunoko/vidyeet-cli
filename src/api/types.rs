/// API通信用の型定義
///
/// Mux Video APIのレスポンスをデシリアライズするための構造体を定義します。
use serde::{Deserialize, Serialize};

/// Direct Uploadレスポンス
///
/// POST /video/v1/uploads のレスポンス型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectUploadResponse {
    pub data: DirectUploadData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectUploadData {
    /// Upload ID
    pub id: String,

    /// アップロード有効期限（秒）
    pub timeout: u64,

    /// アップロードステータス
    pub status: String,

    /// 新規アセット設定
    pub new_asset_settings: NewAssetSettings,

    /// 作成されたアセットID（asset_created状態の場合のみ）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,

    /// エラー情報（errored状態の場合のみ）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<UploadError>,

    /// CORSオリジン
    pub cors_origin: String,

    /// アップロードURL
    pub url: String,

    /// テストアップロードかどうか
    pub test: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAssetSettings {
    pub playback_policy: Vec<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_quality: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<AssetMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator_id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

/// アセットレスポンス
///
/// GET /video/v1/assets/{ASSET_ID} のレスポンス型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetResponse {
    pub data: AssetData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetData {
    /// アセットID
    pub id: String,

    /// ステータス
    pub status: String,

    /// 再生ID
    pub playback_ids: Vec<PlaybackId>,

    /// 動画トラック情報
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracks: Option<Vec<Track>>,

    /// 動画時間（秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,

    /// 作成日時（Unix timestamp）
    pub created_at: String,

    /// アスペクト比
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,

    /// ビデオ品質
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_quality: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackId {
    pub id: String,
    pub policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    #[serde(rename = "type")]
    pub track_type: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
}

/// アセット一覧レスポンス
///
/// GET /video/v1/assets のレスポンス型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetsListResponse {
    pub data: Vec<AssetData>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl DirectUploadResponse {
    /// レスポンスが有効かチェック
    pub fn is_valid(&self) -> bool {
        !self.data.id.is_empty() && !self.data.url.is_empty()
    }

    /// アップロードURLを取得
    pub fn get_upload_url(&self) -> &str {
        &self.data.url
    }

    /// Upload IDを取得
    pub fn get_upload_id(&self) -> &str {
        &self.data.id
    }
}

impl AssetResponse {
    /// 再生URLを構築
    pub fn get_playback_url(&self) -> Option<String> {
        self.data.playback_ids.first().map(|playback_id| {
            format!("https://stream.mux.com/{}.m3u8", playback_id.id)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_upload_response_deserialization() {
        let json = r#"{
            "data": {
                "id": "upload_abc123",
                "timeout": 3600,
                "status": "waiting",
                "new_asset_settings": {
                    "playback_policy": ["public"],
                    "video_quality": "basic"
                },
                "asset_id": null,
                "error": null,
                "cors_origin": "*",
                "url": "https://storage.googleapis.com/...",
                "test": false
            }
        }"#;

        let response: DirectUploadResponse = serde_json::from_str(json).expect("Failed to parse");

        assert_eq!(response.data.id, "upload_abc123");
        assert_eq!(response.data.timeout, 3600);
        assert_eq!(response.data.status, "waiting");
        assert!(response.is_valid());
    }

    #[test]
    fn test_asset_response_playback_url() {
        let response = AssetResponse {
            data: AssetData {
                id: "asset_123".to_string(),
                status: "ready".to_string(),
                playback_ids: vec![PlaybackId {
                    id: "playback_xyz".to_string(),
                    policy: "public".to_string(),
                }],
                tracks: None,
                duration: Some(120.5),
                created_at: "1609869152".to_string(),
                aspect_ratio: Some("16:9".to_string()),
                video_quality: Some("basic".to_string()),
            },
        };

        let url = response.get_playback_url();
        assert!(url.is_some());
        assert_eq!(url.unwrap(), "https://stream.mux.com/playback_xyz.m3u8");
    }

    #[test]
    fn test_assets_list_deserialization() {
        let json = r#"{
            "data": [
                {
                    "id": "asset_1",
                    "status": "ready",
                    "playback_ids": [{"id": "play_1", "policy": "public"}],
                    "created_at": "1609869152"
                }
            ],
            "next_cursor": "cursor_abc"
        }"#;

        let response: AssetsListResponse = serde_json::from_str(json).expect("Failed to parse");

        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].id, "asset_1");
        assert!(response.next_cursor.is_some());
    }
}
