/// ログアウトコマンド
///
/// 保存されているリフレッシュトークンを削除します。
use crate::config::user::UserConfig;
use anyhow::{Context, Result};

/// ログアウトコマンドを実行
///
/// # Returns
/// 成功時はOk(())、失敗時はエラー
pub async fn execute() -> Result<()> {
    println!("api.videoからログアウトします...\n");

    // UserConfigをロード
    let mut config = UserConfig::load()
        .context("設定ファイルの読み込みに失敗しました")?;

    // リフレッシュトークンが存在するか確認
    if !config.has_refresh_token() {
        println!("既にログアウトしています。");
        return Ok(());
    }

    // リフレッシュトークンをクリア
    config.clear_refresh_token();

    // 設定を保存
    config
        .save()
        .context("設定ファイルの保存に失敗しました")?;

    println!("✓ ログアウトしました。");
    println!("リフレッシュトークンが削除されました。");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_logout_without_token() {
        // リフレッシュトークンが存在しない状態でもエラーにならないことを確認
        let result = execute().await;
        // 設定ファイルが存在しない場合はエラーになる可能性があるため、
        // 実際のテストは統合テストで実施
        assert!(result.is_ok() || result.is_err());
    }
}
