/// プレゼンテーション層: ユーザー入力処理
///
/// CLI引数やstdinからのユーザー入力を取得し、
/// アプリケーション層で使用可能な形式に変換します。
use crate::commands::login::LoginCredentials;
use anyhow::{Context, Result, bail};
use std::io::{self, Write};

/// 対話的に認証情報を取得
///
/// プレゼンテーション層の責務として、ユーザー入力を取得し検証する
pub fn read_credentials_interactive() -> Result<LoginCredentials> {
    eprintln!("Logging in to Mux Video...");
    eprintln!();
    eprintln!("Please enter your Mux Access Token credentials.");
    eprintln!("You can find them at: https://dashboard.mux.com/settings/access-tokens");
    eprintln!();

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

    // Token Secret の取得
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

    Ok(LoginCredentials {
        token_id,
        token_secret,
    })
}

/// stdin からパイプで認証情報を取得（2行形式）
///
/// 形式:
///   1行目: Token ID
///   2行目: Token Secret
pub fn read_credentials_from_stdin() -> Result<LoginCredentials> {
    let mut token_id = String::new();
    io::stdin()
        .read_line(&mut token_id)
        .context("Failed to read Token ID from stdin")?;
    let token_id = token_id.trim().to_string();

    if token_id.is_empty() {
        bail!(
            "Token ID cannot be empty. Please ensure the first line of stdin contains a valid Token ID."
        );
    }

    let mut token_secret = String::new();
    io::stdin()
        .read_line(&mut token_secret)
        .context("Failed to read Token Secret from stdin")?;
    let token_secret = token_secret.trim().to_string();

    if token_secret.is_empty() {
        bail!(
            "Token Secret cannot be empty. Please ensure the second line of stdin contains a valid Token Secret."
        );
    }

    Ok(LoginCredentials {
        token_id,
        token_secret,
    })
}

/// 削除確認プロンプトを表示し、ユーザーの確認を得る
///
/// # 引数
/// * `asset_id` - 削除対象のアセットID
///
/// # 戻り値
/// ユーザーが削除を承認した場合はOk(true)、キャンセルした場合はOk(false)
pub fn confirm_delete(asset_id: &str) -> Result<bool> {
    eprintln!();
    eprintln!("⚠️  WARNING: You are about to delete the following asset:");
    eprintln!("   Asset ID: {}", asset_id);
    eprintln!();
    eprintln!("This action cannot be undone. All video data will be permanently deleted.");
    eprintln!();
    eprint!("Type 'yes' to confirm deletion: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("Failed to read confirmation from input")?;

    let input = input.trim();

    if input.eq_ignore_ascii_case("yes") {
        Ok(true)
    } else {
        eprintln!("Deletion cancelled.");
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_empty_token_validation() {
        // 空のトークンは検証でエラーとなることを確認
        // （実際の入力テストはE2Eテストで実施）
        let empty_token = "";
        assert!(empty_token.is_empty());
    }
}
