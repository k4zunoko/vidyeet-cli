use crate::api::auth::AuthManager;
use crate::api::client::ApiClient;
use crate::api::error::InfraError;
use crate::api::types::{DirectUploadResponse, AssetResponse, AssetsListResponse, MuxErrorResponse};
use crate::commands::result::{CommandResult, UploadResult, Mp4Status};
use crate::config::{APP_CONFIG, UserConfig};
use crate::domain::validator;
use crate::domain::progress::{UploadProgress, UploadPhase};
use anyhow::{Context, Result, bail};
use std::time::Duration;
use tokio::time::sleep;

/// アップロードコマンドを実行する
///
/// # 引数
/// * `file_path` - アップロード対象の動画ファイルのパス
/// * `progress_tx` - 進捗通知用チャネルの送信側（オプション）
///
/// # 戻り値
/// 成功・失敗を示すResult<CommandResult>
///
/// # エラー
/// このレイヤーでは anyhow::Result を返し、
/// ドメイン層・インフラ層のエラーを集約する。

pub async fn execute(
    file_path: &str,
    progress_tx: Option<tokio::sync::mpsc::Sender<UploadProgress>>,
) -> Result<CommandResult> {
    // 進捗通知ヘルパー関数
    let notify = |phase: UploadPhase| {
        let tx = progress_tx.clone();
        async move {
            if let Some(tx) = tx {
                let _ = tx.send(UploadProgress::new(phase)).await;
            }
        }
    };

    // ファイル検証開始
    notify(UploadPhase::ValidatingFile {
        file_path: file_path.to_string(),
    }).await;

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

    // ファイル検証完了
    notify(UploadPhase::FileValidated {
        file_name: std::path::Path::new(&validation.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&validation.path)
            .to_string(),
        size_bytes: validation.size,
        format: validation.extension.clone(),
    }).await;

    // 認証マネージャーとAPIクライアントを初期化
    let auth_manager = AuthManager::new(auth.token_id.clone(), auth.token_secret.clone());
    let client = ApiClient::new(APP_CONFIG.api.endpoint.to_string())
        .context("Failed to create API client")?;

    // Direct Upload URL作成開始
    let file_name = std::path::Path::new(&validation.path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&validation.path)
        .to_string();
    
    notify(UploadPhase::CreatingDirectUpload {
        file_name: file_name.clone(),
    }).await;

    // Direct Uploadを開始（制限エラー時に古いものを削除して一度だけ再試行）
    let (upload, deleted_count) = create_direct_upload_with_capacity(&client, &auth_manager).await
        .context("Failed to create Direct Upload (with capacity handling)")?;
    
    // Direct Upload作成完了
    notify(UploadPhase::DirectUploadCreated {
        upload_id: upload.data.id.clone(),
    }).await;
    
    let upload_url = upload.data.url.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Upload URL not found in response"))?;

    // ファイルアップロード開始
    notify(UploadPhase::UploadingFile {
        file_name: file_name.clone(),
        size_bytes: validation.size,
    }).await;

    // ファイルをチャンクアップロード
    upload_file_chunked(&client, upload_url, file_path, validation.size, progress_tx.clone()).await
        .context("Failed to upload file")?;

    // ファイルアップロード完了
    notify(UploadPhase::FileUploaded {
        file_name: file_name.clone(),
        size_bytes: validation.size,
    }).await;

    // アップロードとアセット作成の完了を待機
    // wait_for_upload_completion内で初回のWaitingForAssetメッセージを送信
    let asset = wait_for_upload_completion(&client, &auth_manager, &upload.data.id, progress_tx.clone()).await
        .context("Failed to wait for upload completion")?;

    // 完了
    notify(UploadPhase::Completed {
        asset_id: asset.data.id.clone(),
    }).await;

    // 結果を構造化して返す
    let hls_url = asset.get_playback_url();
    let playback_id = asset.data.playback_ids.first().map(|p| p.id.clone());
    
    // MP4 URLを取得: ready状態なら実URLを、それ以外なら予測URLを生成
    let mp4_url_from_api = asset.get_mp4_playback_url();
    let mp4_status = if mp4_url_from_api.is_some() {
        Mp4Status::Ready
    } else {
        Mp4Status::Generating
    };
    
    // MP4 URLが取得できない場合でも、playback_idがあれば予測URLを生成
    let mp4_url = mp4_url_from_api.or_else(|| {
        playback_id.as_ref().map(|pid| {
            format!("https://stream.mux.com/{}/highest.mp4", pid)
        })
    });

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
            "video_quality": "premium",
            "max_resolution_tier": "2160p",
            "static_renditions": [
                { "resolution": "highest" },
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

/// 容量制限エラーに当たった場合、古いアセットを1つ削除して再試行する
/// 
/// Mux APIの制限系エラーを以下の条件で判定:
/// - HTTP 429 (レート制限): Too Many Requests
/// - HTTP 400/422 (容量制限): メッセージに "limit", "cannot create", "exceeding" を含む
async fn create_direct_upload_with_capacity(
    client: &ApiClient,
    auth_manager: &AuthManager,
) -> Result<(DirectUploadResponse, usize)> {
    match create_direct_upload(client, auth_manager).await {
        Ok(upload) => Ok((upload, 0)),
        Err(e) => {
            let is_limit_error = is_capacity_limit_error(&e);

            if is_limit_error {
                // 最古のアセットを1つ削除して再試行
                let deleted = delete_oldest_assets(client, auth_manager, 1).await?;
                let upload = create_direct_upload(client, auth_manager).await?;
                Ok((upload, deleted))
            } else {
                Err(e)
            }
        }
    }
}

/// エラーが容量/クォータ制限に起因するかを判定
/// 
/// 判定条件:
/// - HTTP 429: レート制限超過（Too Many Requests）
/// - HTTP 400/422 かつ error.type が "invalid_parameters" かつ
///   メッセージに "limited to" + "assets" を含む: 容量制限エラー
fn is_capacity_limit_error(error: &anyhow::Error) -> bool {
    // InfraError::Apiの場合、ステータスコードとメッセージを確認
    if let Some(infra_err) = error.downcast_ref::<InfraError>() {
        if let InfraError::Api { status_code, message, .. } = infra_err {
            // HTTP 429はレート制限
            if matches!(status_code, Some(429)) {
                return true;
            }
            
            // HTTP 400/422の場合、JSONエラーレスポンスをパースして詳細に判定
            if matches!(status_code, Some(400 | 422)) {
                if let Ok(mux_error) = serde_json::from_str::<MuxErrorResponse>(message) {
                    // error.typeが"invalid_parameters"でも、メッセージで容量制限を確認
                    if mux_error.error.error_type == "invalid_parameters" {
                        // メッセージに"limited to"と"assets"の両方が含まれる場合のみ制限エラー
                        let messages_text = mux_error.error.messages.join(" ").to_lowercase();
                        return messages_text.contains("limited to") && messages_text.contains("assets");
                    }
                }
            }
        }
    }
    false
}

/// 最も古いアセットからcount件削除
///
/// Mux APIは新しいものから古いものの順（降順）でアセットを返すため、
/// created_atでソートして最も古いアセットを特定します。
async fn delete_oldest_assets(
    client: &ApiClient,
    auth_manager: &AuthManager,
    count: usize,
) -> Result<usize> {
    let auth_header = auth_manager.get_auth_header();
    let response = client
        .get("/video/v1/assets?limit=100", Some(&auth_header))
        .await
        .context("Failed to fetch assets list for deletion")?;

    let response = ApiClient::check_response(response, "/video/v1/assets").await?;
    let assets_list: AssetsListResponse = ApiClient::parse_json(response).await?;

    // created_atでソートして最も古いものを特定（昇順）
    let mut assets_sorted = assets_list.data;
    assets_sorted.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    let delete_targets = assets_sorted.iter().take(count);
    let mut deleted = 0usize;
    for asset in delete_targets {
        let resp = client
            .delete(&format!("/video/v1/assets/{}", asset.id), Some(&auth_header))
            .await
            .context(format!("Failed to delete asset {}", asset.id))?;
        ApiClient::check_response(resp, &format!("/video/v1/assets/{}", asset.id)).await?;
        deleted += 1;
    }

    Ok(deleted)
}

/// ファイルをDirect Upload URLにアップロード（従来の一括アップロード、未使用）
#[allow(dead_code)]
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

/// ファイルをチャンク分割してDirect Upload URLにアップロード
///
/// Mux Direct Uploadの推奨方式（UpChunk互換）で、大きなファイルを
/// 256KiBの倍数のチャンクに分割してアップロードします。
///
/// # 設計
/// - チャンクサイズ: 32MB（APP_CONFIG.upload.chunk_size）
/// - Content-Rangeヘッダー: `bytes {start}-{end}/{total}`
/// - 進捗通知: チャンク完了ごとに UploadingChunk イベントを送信
/// - リトライ: 指数バックオフで最大3回
/// - レスポンス: 308（継続）、200/201（完了）
///
/// # 引数
/// * `client` - APIクライアント
/// * `upload_url` - Direct Upload URL
/// * `file_path` - アップロード対象ファイルのパス
/// * `total_size` - ファイルの総サイズ（バイト）
/// * `progress_tx` - 進捗通知チャネル
async fn upload_file_chunked(
    client: &ApiClient,
    upload_url: &str,
    file_path: &str,
    total_size: u64,
    progress_tx: Option<tokio::sync::mpsc::Sender<UploadProgress>>,
) -> Result<()> {
    use tokio::io::AsyncReadExt;
    
    let chunk_size = APP_CONFIG.upload.chunk_size;
    let total_chunks = ((total_size as f64) / (chunk_size as f64)).ceil() as usize;
    
    // ファイルを開く
    let mut file = tokio::fs::File::open(file_path)
        .await
        .context("Failed to open file for chunked upload")?;
    
    // Content-Typeを推定
    let content_type = std::path::Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| APP_CONFIG.upload.get_content_type(ext))
        .unwrap_or("application/octet-stream");
    
    let mut bytes_sent: u64 = 0;
    let mut current_chunk = 0;
    
    loop {
        current_chunk += 1;
        
        // チャンクサイズ分のバッファを用意（最終チャンクは残りサイズ）
        let remaining = total_size - bytes_sent;
        let this_chunk_size = if remaining < chunk_size as u64 {
            remaining as usize
        } else {
            chunk_size
        };
        
        if this_chunk_size == 0 {
            break; // 全て送信完了
        }
        
        // チャンクを読み込み
        let mut chunk_buffer = vec![0u8; this_chunk_size];
        file.read_exact(&mut chunk_buffer)
            .await
            .context("Failed to read chunk from file")?;
        
        // Content-Rangeヘッダーを構築
        let byte_start = bytes_sent;
        let byte_end = bytes_sent + this_chunk_size as u64 - 1;
        let content_range = format!("bytes {}-{}/{}", byte_start, byte_end, total_size);
        
        // チャンクをアップロード（リトライ付き）
        upload_chunk_with_retry(
            client,
            upload_url,
            chunk_buffer,
            &content_range,
            content_type,
        ).await?;
        
        bytes_sent += this_chunk_size as u64;
        
        // 進捗通知
        if let Some(ref tx) = progress_tx {
            let _ = tx.send(UploadProgress::new(UploadPhase::UploadingChunk {
                current_chunk,
                total_chunks,
                bytes_sent,
                total_bytes: total_size,
            })).await;
        }
    }
    
    Ok(())
}

/// チャンクを指数バックオフでリトライしながらアップロード
///
/// # 引数
/// * `client` - APIクライアント
/// * `upload_url` - Direct Upload URL
/// * `chunk_data` - チャンクのバイトデータ
/// * `content_range` - Content-Rangeヘッダー値
/// * `content_type` - Content-Type
async fn upload_chunk_with_retry(
    client: &ApiClient,
    upload_url: &str,
    chunk_data: Vec<u8>,
    content_range: &str,
    content_type: &str,
) -> Result<()> {
    let max_retries = APP_CONFIG.upload.max_retries;
    let backoff_base_ms = APP_CONFIG.upload.backoff_base_ms;
    
    for attempt in 0..max_retries {
        match upload_chunk(client, upload_url, &chunk_data, content_range, content_type).await {
            Ok(_) => return Ok(()),
            Err(e) if attempt < max_retries - 1 => {
                // 指数バックオフ: 1秒、2秒、4秒...
                let backoff_ms = backoff_base_ms * (2_u64.pow(attempt));
                eprintln!("Chunk upload failed (attempt {}/{}), retrying in {}ms: {}", 
                    attempt + 1, max_retries, backoff_ms, e);
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }
            Err(e) => {
                return Err(e).context(format!(
                    "Chunk upload failed after {} attempts", max_retries
                ));
            }
        }
    }
    
    bail!("Chunk upload failed after {} retries", max_retries)
}

/// 単一チャンクをアップロード
///
/// # レスポンスコード
/// - 308: Resume Incomplete（継続中）
/// - 200/201: Success（完了）
async fn upload_chunk(
    _client: &ApiClient,
    upload_url: &str,
    chunk_data: &[u8],
    content_range: &str,
    content_type: &str,
) -> Result<()> {
    // reqwestクライアントを直接使用してContent-Rangeヘッダーを設定
    let reqwest_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(APP_CONFIG.api.timeout_seconds))
        .build()
        .context("Failed to build reqwest client")?;
    
    let response = reqwest_client
        .put(upload_url)
        .header("Content-Type", content_type)
        .header("Content-Length", chunk_data.len().to_string())
        .header("Content-Range", content_range)
        .body(chunk_data.to_vec())
        .send()
        .await
        .context("Failed to send chunk PUT request")?;
    
    let status = response.status();
    
    // 308 (Resume Incomplete) または 2xx (Success) なら成功
    if status == reqwest::StatusCode::from_u16(308).unwrap() 
        || status.is_success() {
        return Ok(());
    }
    
    // エラーレスポンス
    let error_body = response.text().await.unwrap_or_else(|_| "No error body".to_string());
    bail!("Chunk upload failed with status {}: {}", status, error_body)
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
    progress_tx: Option<tokio::sync::mpsc::Sender<UploadProgress>>,
) -> Result<AssetResponse> {
    let auth_header = auth_manager.get_auth_header();
    let max_iterations = APP_CONFIG.upload.max_wait_secs / APP_CONFIG.upload.poll_interval_secs;
    let start_time = std::time::Instant::now();

    // 初回の待機メッセージを送信
    if let Some(ref tx) = progress_tx {
        let _ = tx.send(UploadProgress::new(UploadPhase::WaitingForAsset {
            upload_id: upload_id.to_string(),
            elapsed_secs: 0,
        })).await;
    }

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
                // まだ処理中 - 待機してから次の進捗通知
                sleep(Duration::from_secs(APP_CONFIG.upload.poll_interval_secs)).await;
                
                // sleep後に経過時間を進捗通知
                if let Some(ref tx) = progress_tx {
                    let elapsed = start_time.elapsed().as_secs();
                    let _ = tx.send(UploadProgress::new(UploadPhase::WaitingForAsset {
                        upload_id: upload_id.to_string(),
                        elapsed_secs: elapsed,
                    })).await;
                }
            }
        }
    }

    bail!("Upload processing timed out after {} seconds", APP_CONFIG.upload.max_wait_secs)
}
