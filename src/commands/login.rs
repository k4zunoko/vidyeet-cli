/// ログインコマンド
///
/// Mux Video APIのAccess Token (ID + Secret)を使用してログインし、
/// 認証情報をconfig.tomlに保存します。
use crate::api::auth::AuthManager;
use crate::commands::result::{CommandResult, LoginResult};
use crate::config::user::UserConfig;
use anyhow::{Context, Result, bail};
use std::io::{self, Write, IsTerminal};

/// 認証情報の入力方法
#[derive(Debug, Clone, Copy)]
pub enum InputMethod {
    /// 対話的入力（TTY必須）
    Interactive,
    /// stdin からパイプ入力（2行形式: token_id, token_secret）
    Stdin,
}

/// ログインコマンドを実行
///
/// # Arguments
/// * `input_method` - 認証情報の入力方法
///
/// # Returns
/// 成功時はOk(CommandResult)、失敗時はエラー
pub async fn execute(input_method: InputMethod) -> Result<CommandResult> {
    // 既存の設定を確認
    let mut config = UserConfig::load()
        .context("Failed to load configuration file")?;
    
    let was_logged_in = config.has_auth();

    // 入力方法に応じて認証情報を取得
    let (token_id, token_secret) = match input_method {
        InputMethod::Interactive => {
            // TTYチェック: 対話的入力はTTYが必要
            if !io::stdin().is_terminal() {
                bail!("Interactive input requires a TTY. Use '--stdin' flag for non-interactive input.");
            }

            // 既存ログイン時の警告（対話的フローの一部）
            if was_logged_in {
                eprintln!("Note: You are already logged in. Entering new credentials will overwrite the existing ones.");
                eprintln!();
            }

            read_credentials_interactive()?
        }
        InputMethod::Stdin => {
            read_credentials_from_stdin()?
        }
    };

    // 認証マネージャーを作成
    let auth_manager = AuthManager::new(token_id.clone(), token_secret.clone());

    // 認証情報をテスト
    auth_manager
        .test_credentials()
        .await
        .context("Authentication failed. Please verify your Token ID and Secret are correct.")?;

    // 認証情報を保存
    config.set_auth(token_id, token_secret);
    
    config
        .save()
        .context("Failed to save configuration file")?;

    Ok(CommandResult::Login(LoginResult { was_logged_in }))
}

/// 対話的に認証情報を取得（TTY必須）
fn read_credentials_interactive() -> Result<(String, String)> {
    // Token IDの取得
    eprint!("Access Token ID: ");
    io::stdout().flush()?;
    let mut token_id = String::new();
    io::stdin()
        .read_line(&mut token_id)
        .context("Failed to read Token ID from input")?;
    let token_id = token_id.trim().to_string();

    if token_id.is_empty() {
        bail!("Token ID cannot be empty. Please provide a valid Token ID.");
    }

    // Token Secret の取得（エコーなし）
    eprint!("Access Token Secret: ");
    io::stdout().flush()?;
    let mut token_secret = String::new();
    io::stdin()
        .read_line(&mut token_secret)
        .context("Failed to read Token Secret from input")?;
    let token_secret = token_secret.trim().to_string();

    if token_secret.is_empty() {
        bail!("Token Secret cannot be empty. Please provide a valid Token Secret.");
    }

    Ok((token_id, token_secret))
}

/// stdin からパイプで認証情報を取得（2行形式）
/// 
/// 形式:
///   1行目: Token ID
///   2行目: Token Secret
fn read_credentials_from_stdin() -> Result<(String, String)> {
    let mut token_id = String::new();
    io::stdin()
        .read_line(&mut token_id)
        .context("Failed to read Token ID from stdin")?;
    let token_id = token_id.trim().to_string();

    if token_id.is_empty() {
        bail!("Token ID cannot be empty.");
    }

    let mut token_secret = String::new();
    io::stdin()
        .read_line(&mut token_secret)
        .context("Failed to read Token Secret from stdin")?;
    let token_secret = token_secret.trim().to_string();

    if token_secret.is_empty() {
        bail!("Token Secret cannot be empty.");
    }

    Ok((token_id, token_secret))
}

