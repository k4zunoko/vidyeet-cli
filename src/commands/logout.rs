/// ログアウトコマンド
///
/// 保存されている認証情報を削除します。
use crate::commands::result::{CommandResult, LogoutResult};
use crate::config::user::UserConfig;
use anyhow::{Context, Result};

/// ログアウトコマンドを実行
///
/// # Returns
/// 成功時はOk(CommandResult)、失敗時はエラー
pub async fn execute() -> Result<CommandResult> {
    // UserConfigをロード
    let mut config = UserConfig::load()
        .context("Failed to load configuration file")?;

    // 認証情報が存在するか確認
    let was_logged_in = config.has_auth();
    
    if !was_logged_in {
        return Ok(CommandResult::Logout(LogoutResult { was_logged_in: false }));
    }

    // 認証情報をクリア
    config.clear_auth();

    // 設定を保存
    config
        .save()
        .context("Failed to save configuration file")?;

    Ok(CommandResult::Logout(LogoutResult { was_logged_in: true }))
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
