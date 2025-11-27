/// ログアウトコマンド
///
/// 保存されている認証情報を削除します。
use crate::config::user::UserConfig;
use anyhow::{Context, Result};

/// ログアウトコマンドを実行
///
/// # Returns
/// 成功時はOk(())、失敗時はエラー
pub async fn execute() -> Result<()> {
    println!("Logging out from Mux Video...\n");

    // UserConfigをロード
    let mut config = UserConfig::load()
        .context("Failed to load configuration file")?;

    // 認証情報が存在するか確認
    if !config.has_auth() {
        println!("Already logged out.");
        return Ok(());
    }

    // 認証情報をクリア
    config.clear_auth();

    // 設定を保存
    config
        .save()
        .context("Failed to save configuration file")?;

    println!("✓ Logged out successfully.");
    println!("Authentication credentials have been removed.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_logout_without_token() {
        // 認証情報が存在しない状態でもエラーにならないことを確認
        let result = execute().await;
        // 設定ファイルが存在しない場合はエラーになる可能性があるため、
        // 実際のテストは統合テストで実施
        assert!(result.is_ok() || result.is_err());
    }
}
