use crate::config::{APP_CONFIG, UserConfig};
use crate::domain::validator;
use anyhow::{Context, Result};

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

pub fn execute(file_path: &str) -> Result<()> {
    // ユーザー設定を読み込み（自動検証される）
    let user_config = UserConfig::load()
        .context("Failed to load user configuration. Please check your config.toml file.")?;

    // APIキー取得
    let api_key = user_config
        .api_key
        .as_ref()
        .context("API key is not configured")?;

    // ドメイン層のバリデーションを実行
    // DomainError は自動的に anyhow::Error に変換される
    let validation =
        validator::validate_upload_file(file_path).context("File validation failed")?;

    println!("File validated successfully:");
    println!("  Path: {}", validation.path);
    println!(
        "  Size: {} bytes ({:.2} MB)",
        validation.size,
        validation.size as f64 / 1024.0 / 1024.0
    );
    println!("  Format: {}", validation.extension);

    // AppConfigから設定を直接取得
    println!("\nUsing configuration:");
    println!("  API Endpoint: {}", APP_CONFIG.api.endpoint);
    println!("  Timeout: {}s", APP_CONFIG.api.timeout_seconds);
    println!(
        "  Max File Size: {} bytes ({} MB)",
        APP_CONFIG.upload.max_file_size,
        APP_CONFIG.upload.max_file_size / 1024 / 1024
    );
    println!("  API Key: {}...", &api_key[..10.min(api_key.len())]);

    // TODO: インフラ層 - Streamable APIクライアントの初期化
    // TODO: インフラ層 - ファイルをStreamableにアップロード
    // TODO: アップロードされた動画のURLを返す

    println!("\n[TODO] Upload to Streamable API");

    Ok(())
}
