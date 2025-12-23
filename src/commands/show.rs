use crate::api::auth::AuthManager;
use crate::api::client::ApiClient;
use crate::api::types::AssetResponse;
use crate::commands::result::{CommandResult, ShowResult};
use crate::config::{APP_CONFIG, UserConfig};
use anyhow::{Context, Result};

/// アセット詳細を表示するコマンドを実行する
///
/// Mux APIから指定されたアセットIDの詳細情報を取得します。
///
/// # 引数
/// * `asset_id` - 取得するアセットのID
///
/// # 戻り値
/// 成功・失敗を示すResult<CommandResult>
///
/// # エラー
/// アプリケーション層としてanyhow::Resultを返し、
/// 設定・認証・インフラ層のエラーを集約します。
pub async fn execute(asset_id: &str) -> Result<CommandResult> {
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

    // アセット詳細を取得
    let asset = fetch_asset(&client, &auth_manager, asset_id)
        .await
        .context("Failed to fetch asset details")?;

    // ShowResultを構築
    let result = ShowResult {
        asset_id: asset.data.id.clone(),
        status: asset.data.status.clone(),
        duration: asset.data.duration,
        aspect_ratio: asset.data.aspect_ratio.clone(),
        video_quality: asset.data.video_quality.clone(),
        created_at: asset.data.created_at.clone(),
        playback_ids: asset.data.playback_ids.clone(),
        hls_url: asset.get_playback_url(),
        mp4_url: asset.get_mp4_playback_url(),
        tracks: asset.data.tracks.clone(),
        static_renditions: asset.data.static_renditions.clone(),
    };

    Ok(CommandResult::Show(result))
}

/// Mux APIからアセット詳細を取得
///
/// # 引数
/// * `client` - APIクライアント
/// * `auth_manager` - 認証マネージャー
/// * `asset_id` - アセットID
///
/// # 戻り値
/// アセット詳細のレスポンス
async fn fetch_asset(
    client: &ApiClient,
    auth_manager: &AuthManager,
    asset_id: &str,
) -> Result<AssetResponse> {
    let auth_header = auth_manager.get_auth_header();
    let endpoint = format!("/video/v1/assets/{}", asset_id);

    let response = client
        .get(&endpoint, Some(&auth_header))
        .await
        .context("Failed to fetch asset details")?;

    let response = ApiClient::check_response(response, &endpoint).await?;
    let asset_response: AssetResponse = ApiClient::parse_json(response).await?;

    Ok(asset_response)
}
