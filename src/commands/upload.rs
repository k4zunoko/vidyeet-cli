use crate::api::auth::AuthManager;
use crate::api::client::ApiClient;
use crate::api::types::{DirectUploadResponse, AssetResponse, AssetsListResponse};
use crate::config::{APP_CONFIG, UserConfig};
use crate::domain::validator;
use anyhow::{Context, Result, bail};
use std::io::IsTerminal;
use std::time::Duration;
use tokio::time::sleep;

const MAX_FREE_TIER_VIDEOS: usize = 10;
const UPLOAD_POLL_INTERVAL_SECS: u64 = 2;
const UPLOAD_MAX_WAIT_SECS: u64 = 300; // 5分

/// アップロードコマンドを実行する
///
/// # 引数
/// * `file_path` - アップロード対象の動画ファイルのパス
///
/// # 戻り値
/// 成功・失敗を示すResult
///
/// # エラー
/// このレイヤーでは anyhow::Result を返し、
/// ドメイン層・インフラ層のエラーを集約する。

pub async fn execute(file_path: &str) -> Result<()> {
    // 人間向けプログレス表示（stderr）
    eprintln!("Uploading to Mux Video...\n");

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

    eprintln!("File validated successfully:");
    eprintln!("  Path: {}", validation.path);
    eprintln!(
        "  Size: {} bytes ({:.2} MB)",
        validation.size,
        validation.size as f64 / 1024.0 / 1024.0
    );
    eprintln!("  Format: {}", validation.extension);

    // 認証マネージャーとAPIクライアントを初期化
    let auth_manager = AuthManager::new(auth.token_id.clone(), auth.token_secret.clone());
    let client = ApiClient::new(APP_CONFIG.api.endpoint.to_string())
        .context("Failed to create API client")?;

    // 動画数制限のチェックと管理（10本以上ある場合は古いものを削除）
    eprintln!("\nChecking video count...");
    manage_video_limit(&client, &auth_manager).await
        .context("Failed to manage video limit")?;

    // Direct Uploadを開始
    eprintln!("\nCreating Direct Upload...");
    let upload = create_direct_upload(&client, &auth_manager).await
        .context("Failed to create Direct Upload")?;

    eprintln!("✓ Direct Upload created: {}", upload.data.id);
    
    let upload_url = upload.data.url.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Upload URL not found in response"))?;
    eprintln!("  Upload URL: {}", upload_url);

    // ファイルをアップロード
    eprintln!("\nUploading file...");
    upload_file(&client, upload_url, file_path).await
        .context("Failed to upload file")?;

    eprintln!("✓ File uploaded successfully");

    // アップロードとアセット作成の完了を待機
    eprintln!("\nWaiting for asset creation...");
    let asset = wait_for_upload_completion(&client, &auth_manager, &upload.data.id).await
        .context("Failed to wait for upload completion")?;

    // 人間向け結果表示（stderr）
    eprintln!("\n✓ Upload completed successfully!");
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    eprintln!("  Asset ID: {}", asset.data.id);
    
    // HLS再生URL（すぐに利用可能）
    if let Some(playback_url) = asset.get_playback_url() {
        eprintln!("\n  🎬 HLS Streaming URL (ready now):");
        eprintln!("     {}", playback_url);
    }
    
    // MP4再生URL（バックグラウンド生成中または完成済み）
    eprintln!("\n  📦 MP4 Download URL:");
    if let Some(mp4_url) = asset.get_mp4_playback_url() {
        eprintln!("     Status: ✓ Ready");
        eprintln!("     {}", mp4_url);
    } else {
        // MP4構築URLを先に提供（生成完了後に利用可能）
        if let Some(playback_id) = asset.data.playback_ids.first() {
            let potential_mp4_url = format!("https://stream.mux.com/{}/highest.mp4", playback_id.id);
            eprintln!("     Status: ⏳ Generating...");
            eprintln!("     {}", potential_mp4_url);
            eprintln!("\n     Note: MP4 file is being generated in the background (usually 2-5 minutes).");
            eprintln!("           The URL above will be available once generation completes.");
            eprintln!("           You can start streaming with HLS URL immediately!");
        } else {
            eprintln!("     Status: Pending (playback ID not yet available)");
        }
    }
    
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 機械可読な結果をstdoutに出力（パイプライン/リダイレクト時のみ）
    // TTY（ターミナル）接続時はstderrの人間向け出力のみとする
    if !std::io::stdout().is_terminal() {
        let mp4_url = asset.data.playback_ids.first()
            .map(|pb| format!("https://stream.mux.com/{}/highest.mp4", pb.id))
            .unwrap_or_else(|| "N/A".to_string());
        
        let result = serde_json::json!({
            "success": true,
            "asset_id": asset.data.id,
            "hls_url": asset.get_playback_url(),
            "mp4_url": mp4_url,
            "mp4_status": if asset.get_mp4_playback_url().is_some() { "ready" } else { "generating" }
        });
        println!("{}", serde_json::to_string(&result)?);
    }

    Ok(())
}

/// 無料枠では10本までしか動画を保存できないため、
/// 既に10本以上ある場合は最も古いものを削除します。
async fn manage_video_limit(
    client: &ApiClient,
    auth_manager: &AuthManager,
) -> Result<()> {
    let auth_header = auth_manager.get_auth_header();
    
    // 現在のアセット一覧を取得
    let response = client
        .get("/video/v1/assets?limit=100", Some(&auth_header))
        .await
        .context("Failed to fetch assets list")?;

    let response = ApiClient::check_response(response, "/video/v1/assets").await?;
    let assets_list: AssetsListResponse = ApiClient::parse_json(response).await?;

    let current_count = assets_list.data.len();
    eprintln!("  Current video count: {}/{}", current_count, MAX_FREE_TIER_VIDEOS);

    // 10本以上ある場合は古いものから削除
    if current_count >= MAX_FREE_TIER_VIDEOS {
        eprintln!("  Limit reached. Deleting oldest videos...");
        
        let delete_count = current_count - MAX_FREE_TIER_VIDEOS + 1;
        
        // 最初のN個（最も古い）を削除
        for asset in assets_list.data.iter().take(delete_count) {
            eprintln!("  Deleting asset: {}", asset.id);
            
            let response = client
                .delete(&format!("/video/v1/assets/{}", asset.id), Some(&auth_header))
                .await
                .context(format!("Failed to delete asset {}", asset.id))?;
            
            ApiClient::check_response(response, &format!("/video/v1/assets/{}", asset.id)).await?;
        }
        
        eprintln!("  ✓ Deleted {} old video(s)", delete_count);
    }

    Ok(())
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
    let max_iterations = UPLOAD_MAX_WAIT_SECS / UPLOAD_POLL_INTERVAL_SECS;

    for i in 0..max_iterations {
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
                if i % 5 == 0 {
                    eprintln!("  Status: {} (waiting...)", upload.data.status);
                }
                sleep(Duration::from_secs(UPLOAD_POLL_INTERVAL_SECS)).await;
            }
        }
    }

    bail!("Upload processing timed out after {} seconds", UPLOAD_MAX_WAIT_SECS)
}
