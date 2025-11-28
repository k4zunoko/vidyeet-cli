use crate::api::auth::AuthManager;
use crate::api::client::ApiClient;
use crate::api::types::{DirectUploadResponse, AssetResponse, AssetsListResponse};
use crate::commands::result::{CommandResult, UploadResult, Mp4Status};
use crate::config::{APP_CONFIG, UserConfig};
use crate::domain::validator;
use anyhow::{Context, Result, bail};
use std::time::Duration;
use tokio::time::sleep;

/// アップロードコマンドを実行する
///
/// # 引数
/// * `file_path` - アップロード対象の動画ファイルのパス
///
/// # 戻り値
/// 成功・失敗を示すResult<CommandResult>
///
/// # エラー
/// このレイヤーでは anyhow::Result を返し、
/// ドメイン層・インフラ層のエラーを集約する。

pub async fn execute(file_path: &str) -> Result<CommandResult> {

    // ユーザー設定を読み込み
    let user_config = UserConfig::load()
        .context("Failed to load user configuration. Please check your config.toml file.")?;

    // 認証情報を取得
    let auth = user_config
        .get_auth()
        .context("Authentication credentials not found. Please run 'vidyeet login' first.")?;

    // ドメイン層のバリデーションを実行
    let validation =
        validator::validate_upload_file(file_path).context("File validation failed")?;

    // 認証マネージャーとAPIクライアントを初期化
    let auth_manager = AuthManager::new(auth.token_id.clone(), auth.token_secret.clone());
    let client = ApiClient::new(APP_CONFIG.api.endpoint.to_string())
        .context("Failed to create API client")?;

    // 動画数制限のチェックと管理（10本以上ある場合は古いものを削除）
    let deleted_count = manage_video_limit(&client, &auth_manager).await
        .context("Failed to manage video limit")?;

    // Direct Uploadを開始
    let upload = create_direct_upload(&client, &auth_manager).await
        .context("Failed to create Direct Upload")?;
    
    let upload_url = upload.data.url.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Upload URL not found in response"))?;

    // ファイルをアップロード
    upload_file(&client, upload_url, file_path).await
        .context("Failed to upload file")?;

    // アップロードとアセット作成の完了を待機
    let asset = wait_for_upload_completion(&client, &auth_manager, &upload.data.id).await
        .context("Failed to wait for upload completion")?;

    // 結果を構造化して返す
    let hls_url = asset.get_playback_url();
    let mp4_url = asset.get_mp4_playback_url();
    let playback_id = asset.data.playback_ids.first().map(|p| p.id.clone());
    let mp4_status = if mp4_url.is_some() {
        Mp4Status::Ready
    } else {
        Mp4Status::Generating
    };

    Ok(CommandResult::Upload(UploadResult {
        asset_id: asset.data.id,
        playback_id,
        hls_url,
        mp4_url,
        mp4_status,
        file_path: validation.path,
        file_size: validation.size,
        file_format: validation.extension,
        deleted_old_videos: deleted_count,
    }))
}

/// 無料枠では10本までしか動画を保存できないため、
/// 既に10本以上ある場合は最も古いものを削除します。
/// 
/// # Returns
/// 削除した動画の数
async fn manage_video_limit(
    client: &ApiClient,
    auth_manager: &AuthManager,
) -> Result<usize> {
    let auth_header = auth_manager.get_auth_header();
    
    // 現在のアセット一覧を取得
    let response = client
        .get("/video/v1/assets?limit=100", Some(&auth_header))
        .await
        .context("Failed to fetch assets list")?;

    let response = ApiClient::check_response(response, "/video/v1/assets").await?;
    let assets_list: AssetsListResponse = ApiClient::parse_json(response).await?;

    let current_count = assets_list.data.len();

    // 10本以上ある場合は古いものから削除
    if current_count >= APP_CONFIG.upload.max_free_tier_videos {
        let delete_count = current_count - APP_CONFIG.upload.max_free_tier_videos + 1;
        
        // 最初のN個（最も古い）を削除
        for asset in assets_list.data.iter().take(delete_count) {
            let response = client
                .delete(&format!("/video/v1/assets/{}", asset.id), Some(&auth_header))
                .await
                .context(format!("Failed to delete asset {}", asset.id))?;
            
            ApiClient::check_response(response, &format!("/video/v1/assets/{}", asset.id)).await?;
        }
        
        Ok(delete_count)
    } else {
        Ok(0)
    }
}

/// Direct Uploadを作成
async fn create_direct_upload(
    client: &ApiClient,
    auth_manager: &AuthManager,
) -> Result<DirectUploadResponse> {
    let auth_header = auth_manager.get_auth_header();
    
    // Direct Upload作成リクエスト
    let request_body = serde_json::json!({
        "new_asset_settings": {
            "playback_policies": ["public"],
            "static_renditions": [
                { "resolution": "highest" },   // 最大解像度のMP4
            ]
        }
    });

    let response = client
        .post(
            "/video/v1/uploads",
            &request_body,
            Some(&auth_header),
        )
        .await
        .context("Failed to create Direct Upload")?;

    let response = ApiClient::check_response(response, "/video/v1/uploads").await?;
    let upload: DirectUploadResponse = ApiClient::parse_json(response).await?;

    Ok(upload)
}

/// ファイルをDirect Upload URLにアップロード
async fn upload_file(
    client: &ApiClient,
    upload_url: &str,
    file_path: &str,
) -> Result<()> {
    // ファイルを読み込み
    let file_content = tokio::fs::read(file_path)
        .await
        .context("Failed to read file")?;

    // ファイルの拡張子からContent-Typeを推定（APP_CONFIGで一元管理）
    let content_type = std::path::Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| APP_CONFIG.upload.get_content_type(ext))
        .unwrap_or("application/octet-stream");

    // PUTリクエストでファイルをアップロード
    let response = client
        .put(upload_url, file_content, content_type)
        .await
        .context("Failed to PUT file to upload URL")?;

    ApiClient::check_response(response, upload_url).await?;

    Ok(())
}

/// アップロードとアセット作成の完了を待機
///
/// Direct Uploadのステータスをポーリングし、`asset_created`状態になるまで待機します。
/// この時点でHLS再生が可能ですが、MP4 static renditionはバックグラウンドで
/// 生成中の場合があります。
///
/// # 設計意図
/// CLIの役割は「アップロードとアセット作成の完了確認」までとし、
/// MP4生成（数分かかる可能性）は待たずにMux側に任せます。
/// これにより、ユーザーはすぐにHLS URLでストリーミングを開始でき、
/// MP4は後で生成完了時にアクセスできます。
async fn wait_for_upload_completion(
    client: &ApiClient,
    auth_manager: &AuthManager,
    upload_id: &str,
) -> Result<AssetResponse> {
    let auth_header = auth_manager.get_auth_header();
    let max_iterations = APP_CONFIG.upload.max_wait_secs / APP_CONFIG.upload.poll_interval_secs;

    for _i in 0..max_iterations {
        // Upload情報を取得
        let response = client
            .get(
                &format!("/video/v1/uploads/{}", upload_id),
                Some(&auth_header),
            )
            .await
            .context("Failed to fetch upload status")?;

        let response = ApiClient::check_response(response, &format!("/video/v1/uploads/{}", upload_id)).await?;
        let upload: DirectUploadResponse = ApiClient::parse_json(response).await?;

        match upload.data.status.as_str() {
            "asset_created" => {
                // Asset IDを取得
                if let Some(asset_id) = upload.data.asset_id {
                    // Assetの詳細を取得
                    let asset_response = client
                        .get(
                            &format!("/video/v1/assets/{}", asset_id),
                            Some(&auth_header),
                        )
                        .await
                        .context("Failed to fetch asset details")?;

                    let asset_response = ApiClient::check_response(asset_response, &format!("/video/v1/assets/{}", asset_id)).await?;
                    let asset: AssetResponse = ApiClient::parse_json(asset_response).await?;

                    return Ok(asset);
                } else {
                    bail!("Upload completed but asset_id is missing");
                }
            }
            "errored" => {
                bail!("Upload failed with error status");
            }
            "cancelled" => {
                bail!("Upload was cancelled");
            }
            "timed_out" => {
                bail!("Upload timed out");
            }
            _ => {
                // まだ処理中 - 待機
                sleep(Duration::from_secs(APP_CONFIG.upload.poll_interval_secs)).await;
            }
        }
    }

    bail!("Upload processing timed out after {} seconds", APP_CONFIG.upload.max_wait_secs)
}
