/// ログインコマンド
///
/// Mux Video APIのAccess Token (ID + Secret)を使用してログインし、
/// 認証情報をconfig.tomlに保存します。
use crate::api::auth::AuthManager;
use crate::config::user::UserConfig;
use anyhow::{Context, Result};
use std::io::{self, Write};

/// ログインコマンドを実行
///
/// # Returns
/// 成功時はOk(())、失敗時はエラー
pub async fn execute() -> Result<()> {
    eprintln!("Logging in to Mux Video...\n");
    eprintln!("Please enter your Mux Access Token credentials.");
    eprintln!("You can find them at: https://dashboard.mux.com/settings/access-tokens\n");

    // Token IDの取得
    print!("Access Token ID: ");
    io::stdout().flush()?;
    let mut token_id = String::new();
    io::stdin()
        .read_line(&mut token_id)
        .context("Failed to read Token ID from input")?;
    let token_id = token_id.trim().to_string();

    if token_id.is_empty() {
        anyhow::bail!("Token ID cannot be empty. Please provide a valid Token ID.");
    }

    // Token Secretの取得
    print!("Access Token Secret: ");
    io::stdout().flush()?;
    let mut token_secret = String::new();
    io::stdin()
        .read_line(&mut token_secret)
        .context("Failed to read Token Secret from input")?;
    let token_secret = token_secret.trim().to_string();

    if token_secret.is_empty() {
        anyhow::bail!("Token Secret cannot be empty. Please provide a valid Token Secret.");
    }

    // 認証マネージャーを作成
    let auth_manager = AuthManager::new(token_id.clone(), token_secret.clone());

    // 認証情報をテスト
    eprintln!("\nVerifying credentials...");
    auth_manager
        .test_credentials()
        .await
        .context("Authentication failed. Please verify your Token ID and Secret are correct.")?;

    // UserConfigをロードして認証情報を保存
    let mut config = UserConfig::load()
        .context("Failed to load configuration file")?;
    
    config.set_auth(token_id, token_secret);
    
    config
        .save()
        .context("Failed to save configuration file")?;

    eprintln!("\n✓ Login successful!");
    eprintln!("Authentication credentials have been saved.");

    Ok(())
}

