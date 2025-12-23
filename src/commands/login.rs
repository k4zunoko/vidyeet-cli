/// ログインコマンド
///
/// Mux Video APIのAccess Token (ID + Secret)を使用してログインし、
/// 認証情報をconfig.tomlに保存します。
use crate::api::auth::AuthManager;
use crate::commands::result::{CommandResult, LoginResult};
use crate::config::user::UserConfig;
use anyhow::{Context, Result};

/// 認証情報を保持する構造体
///
/// プレゼンテーション層で収集された認証情報を
/// ビジネスロジック層に渡すためのカプセル化
#[derive(Debug, Clone)]
pub struct LoginCredentials {
    pub token_id: String,
    pub token_secret: String,
}

/// ログインコマンドを実行
///
/// # Arguments
/// * `credentials` - 認証情報（Token ID と Token Secret）
///
/// # Returns
/// 成功時はOk(CommandResult)、失敗時はエラー
pub async fn execute(credentials: LoginCredentials) -> Result<CommandResult> {
    // 既存の設定を確認
    let mut config = UserConfig::load().context("Failed to load configuration file")?;

    let was_logged_in = config.has_auth();

    // 認証マネージャーを作成
    let auth_manager = AuthManager::new(
        credentials.token_id.clone(),
        credentials.token_secret.clone(),
    );

    // 認証情報をテスト
    auth_manager
        .test_credentials()
        .await
        .context("Authentication failed. Please verify your Token ID and Secret are correct.")?;

    // 認証情報を保存
    config.set_auth(credentials.token_id, credentials.token_secret);

    config.save().context("Failed to save configuration file")?;

    Ok(CommandResult::Login(LoginResult { was_logged_in }))
}
