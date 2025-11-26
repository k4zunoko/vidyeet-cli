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
    println!("api.videoにログインします...\n");

    // APIキーの取得（引数またはプロンプト）
    #[allow(non_snake_case)] // パターンマッチングであるNoneが警告されるのを抑制
    let api_key = match api_key_arg {
        Some(key) => key,
        None => {
            print!("APIキーを入力してください: ");
            io::stdout().flush()?;
            
            // APIキーを標準入力から読み取り
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .context("APIキーの読み取りに失敗しました")?;
            
            input.trim().to_string()
        }
    };

    if api_key.is_empty() {
        anyhow::bail!("APIキーが空です。有効なAPIキーを入力してください。");
    }

    // 認証マネージャーを作成
    let mut auth_manager = AuthManager::new()
        .context("認証マネージャーの初期化に失敗しました")?;

    // ログイン実行
    println!("認証中...");
    let refresh_token = auth_manager
        .login(&api_key)
        .await
        .context("ログインに失敗しました。APIキーが正しいか確認してください。")?;

    // UserConfigをロードしてリフレッシュトークンを保存
    let mut config = UserConfig::load()
        .context("設定ファイルの読み込みに失敗しました")?;
    
    config.set_refresh_token(refresh_token);
    
    config
        .save()
        .context("設定ファイルの保存に失敗しました")?;

    println!("\n✓ ログインに成功しました！");
    println!("リフレッシュトークンが保存されました。");

    Ok(())
}

