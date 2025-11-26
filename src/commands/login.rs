/// ログインコマンド
///
/// api.videoのAPIキーを使用してログインし、
/// リフレッシュトークンをconfig.tomlに保存します。
use crate::api::auth::AuthManager;
use crate::config::user::UserConfig;
use anyhow::{Context, Result};
use std::io::{self, Write};

/// ログインコマンドを実行
///
/// # Arguments
/// * `api_key_arg` - コマンドライン引数から渡されたAPIキー（オプション）
///
/// # Returns
/// 成功時はOk(())、失敗時はエラー
pub async fn execute(api_key_arg: Option<String>) -> Result<()> {
    println!("Logging in to api.video...\n");

    // APIキーの取得（引数またはプロンプト）
    let api_key = match api_key_arg {
        Some(key) => key,
        std::option::Option::None => {
            print!("Enter your API key: ");
            io::stdout().flush()?;
            
            // APIキーを標準入力から読み取り
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .context("Failed to read API key from input")?;
            
            input.trim().to_string()
        }
    };

    if api_key.is_empty() {
        anyhow::bail!("API key cannot be empty. Please provide a valid API key.");
    }

    // 認証マネージャーを作成
    let mut auth_manager = AuthManager::new()
        .context("Failed to initialize authentication manager")?;

    // ログイン実行
    println!("Authenticating...");
    let refresh_token = auth_manager
        .login(&api_key)
        .await
        .context("Login failed. Please verify your API key is correct.")?;

    // UserConfigをロードしてリフレッシュトークンを保存
    let mut config = UserConfig::load()
        .context("Failed to load configuration file")?;
    
    config.set_refresh_token(refresh_token);
    
    config
        .save()
        .context("Failed to save configuration file")?;

    println!("\n✓ Login successful!");
    println!("Refresh token has been saved.");

    Ok(())
}

