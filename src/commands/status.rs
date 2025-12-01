/// ステータスコマンド
///
/// 現在の認証情報でMux Video APIにアクセスできるか（ログイン状態か）を確認します。
use crate::api::auth::AuthManager;
use crate::commands::result::{CommandResult, StatusResult};
use crate::config::user::UserConfig;
use anyhow::{Context, Result};

/// ステータスコマンドを実行
///
/// # Returns
/// 成功時はOk(CommandResult)、失敗時はエラー
pub async fn execute() -> Result<CommandResult> {
    eprintln!("Checking authentication status...\n");
    
    // 設定を読み込み
    let config = UserConfig::load()
        .context("Failed to load configuration file")?;
    
    // 認証情報の存在を確認
    if !config.has_auth() {
        eprintln!("✗ Not logged in");
        eprintln!("  No authentication credentials found.");
        eprintln!("  Please run 'vidyeet login' to authenticate.");
        
        return Ok(CommandResult::Status(StatusResult {
            is_authenticated: false,
            token_id: None,
        }));
    }
    
    // 認証情報を取得
    let auth = config.get_auth()
        .context("Failed to retrieve authentication credentials")?;
    
    // 認証マネージャーを作成
    let auth_manager = AuthManager::new(auth.token_id.clone(), auth.token_secret.clone());
    
    // 認証情報をテスト
    match auth_manager.test_credentials().await {
        Ok(_) => {
            Ok(CommandResult::Status(StatusResult {
                is_authenticated: true,
                token_id: Some(auth_manager.get_masked_token_id()),
            }))
        }
        Err(e) => {
            eprintln!("✗ Authentication failed");
            eprintln!("  Token ID: {}", auth_manager.get_masked_token_id());
            eprintln!("  Error: {}", e);
            eprintln!("\n  Your credentials may be invalid or expired.");
            eprintln!("  Please run 'vidyeet login' to update your credentials.");
            
            Ok(CommandResult::Status(StatusResult {
                is_authenticated: false,
                token_id: Some(auth_manager.get_masked_token_id()),
            }))
        }
    }
}
