use crate::api::auth::AuthManager;
use crate::api::client::ApiClient;
use crate::api::types::AssetsListResponse;
use crate::commands::result::{CommandResult, ListResult, VideoInfo};
use crate::config::{APP_CONFIG, UserConfig};
use anyhow::{Context, Result};

/// リストコマンドを実行する
///
/// Mux APIから現在投稿中の動画のリストを取得します。
///
/// # 戻り値
/// 成功・失敗を示すResult<CommandResult>
///
/// # エラー
/// アプリケーション層としてanyhow::Resultを返し、
/// 設定・認証・インフラ層のエラーを集約します。
pub async fn execute() -> Result<CommandResult> {
    // ユーザー設定を読み込み
    let user_config = UserConfig::load()
        .context("Failed to load user configuration. Please check your config.toml file.")?;

    // 認証情報を取得
    let auth = user_config
        .get_auth()
        .context("Authentication credentials not found. Please run 'vidyeet login' first.")?;

    // 認証マネージャーとAPIクライアントを初期化
    let auth_manager = AuthManager::new(auth.token_id.clone(), auth.token_secret.clone());
    let client = ApiClient::new(APP_CONFIG.api.endpoint.to_string())
        .context("Failed to create API client")?;

    // アセット一覧を取得
    let assets = fetch_all_assets(&client, &auth_manager).await
        .context("Failed to fetch assets list")?;

    // 動画情報のリストを構築
    let videos: Vec<VideoInfo> = assets
        .data
        .into_iter()
        .map(|asset| {
            let playback_id = asset.playback_ids.first().map(|p| p.id.clone());
            let hls_url = playback_id.as_ref().map(|id| {
                format!("https://stream.mux.com/{}.m3u8", id)
            });
            let mp4_url = playback_id.as_ref().map(|id| {
                format!("https://stream.mux.com/{}/highest.mp4", id)
            });

            VideoInfo {
                asset_id: asset.id,
                status: asset.status,
                playback_id,
                hls_url,
                mp4_url,
                duration: asset.duration,
                created_at: asset.created_at,
                aspect_ratio: asset.aspect_ratio,
            }
        })
        .collect();

    let total_count = videos.len();

    Ok(CommandResult::List(ListResult {
        videos,
        total_count,
    }))
}

/// Mux APIからアセット一覧を取得
///
/// # 引数
/// * `client` - APIクライアント
/// * `auth_manager` - 認証マネージャー
///
/// # 戻り値
/// アセット一覧のレスポンス
async fn fetch_all_assets(
    client: &ApiClient,
    auth_manager: &AuthManager,
) -> Result<AssetsListResponse> {
    let auth_header = auth_manager.get_auth_header();

    let response = client
        .get("/video/v1/assets?limit=100", Some(&auth_header))
        .await
        .context("Failed to fetch assets list")?;

    let response = ApiClient::check_response(response, "/video/v1/assets").await?;
    let assets_list: AssetsListResponse = ApiClient::parse_json(response).await?;

    Ok(assets_list)
}
