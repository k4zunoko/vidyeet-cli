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

    // TODO: Phase 6 - 認証実装後に有効化
    // let refresh_token = user_config
    //     .get_refresh_token()
    //     .context("Token not found. Please run 'vidyeet login' first.")?;
    
    // 現在はダミー値を使用
    let _user_config = user_config; // 未使用警告を回避

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
    // TODO: Phase 6 - 認証実装後にトークン表示を有効化
    // println!("  Refresh Token: {}...", &refresh_token[..10.min(refresh_token.len())]);

    // TODO: インフラ層 - api.video APIクライアントの初期化
    // TODO: インフラ層 - ファイルをapi.videoにアップロード
    // TODO: アップロードされた動画のURLを返す

    println!("\n[TODO] Upload to api.video API");

    Ok(())
}
