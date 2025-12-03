use crate::commands;
use crate::domain::progress::UploadProgress;
use crate::presentation::progress::DisplayProgress;
use crate::presentation::output;
use anyhow::{Context, Result, bail};
use std::io::{self, Write};

/// CLI引数を解析し、適切なコマンドにディスパッチする
pub async fn parse_args(args: &[String]) -> Result<()> {
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    // グローバルフラグ --machine のチェック
    let (machine_output, command_start_index) = if args.len() > 1 && args[1] == "--machine" {
        (true, 2)
    } else {
        (false, 1)
    };

    if args.len() < command_start_index + 1 {
        print_usage();
        return Ok(());
    }

    let command = &args[command_start_index];

    let result = match command.as_str() {
        "login" => {
            // --stdin フラグをチェック
            let use_stdin = args.get(command_start_index + 1).map(|s| s.as_str()) == Some("--stdin");
            
            let credentials = if use_stdin {
                // stdin からパイプで認証情報を取得
                read_credentials_from_stdin()
                    .context("Failed to read credentials from stdin")?
            } else {
                // 対話的入力の場合
                
                // 案内メッセージを表示（プレゼンテーション層の責務）
                eprintln!("Logging in to Mux Video...");
                eprintln!();
                eprintln!("Please enter your Mux Access Token credentials.");
                eprintln!("You can find them at: https://dashboard.mux.com/settings/access-tokens");
                eprintln!();
                
                // 対話的に認証情報を取得
                read_credentials_interactive()
                    .context("Failed to read credentials interactively")?
            };
            
            commands::login::execute(credentials)
                .await
                .context("Login command failed")?
        }
        "logout" => {
            commands::logout::execute()
                .await
                .context("Logout command failed")?
        }
        "status" => {
            commands::status::execute()
                .await
                .context("Status command failed")?
        }
        "list" => {
            commands::list::execute()
                .await
                .context("List command failed")?
        }
        "upload" => {
            let file_path = args
                .get(command_start_index + 1)
                .context("Please specify a file path for upload command")?;
            
            // 進捗通知チャネルを作成
            let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel::<UploadProgress>(32);
            
            // アップロード処理を別タスクで開始
            let upload_handle = tokio::spawn({
                let file_path = file_path.to_string();
                async move {
                    commands::upload::execute(&file_path, Some(progress_tx)).await
                }
            });
            
            // 進捗受信ループ（プレゼンテーション層の責務）
            // タイムアウトを設定して無限待機を防ぐ
            use tokio::time::{timeout, Duration};
            let progress_timeout = Duration::from_secs(350); // アップロード最大300秒 + バッファ50秒
            
            loop {
                match timeout(progress_timeout, progress_rx.recv()).await {
                    Ok(Some(progress)) => {
                        if !machine_output {
                            // ドメイン層の型をプレゼンテーション層の型に変換（借用）
                            // Option<DisplayProgress>を返すため、表示が必要な場合のみ出力
                            if let Some(display_progress) = Option::<DisplayProgress>::from(&progress) {
                                // 人間向け進捗表示（stderr）
                                display_upload_progress(&display_progress);
                            }
                            // Noneの場合は表示を抑制（10秒未満の経過時間更新など）
                        }
                        // --machine フラグでは進捗メッセージを抑制
                    }
                    Ok(std::option::Option::None) => {
                        // チャネルがクローズされた（正常終了）
                        break;
                    }
                    Err(_) => {
                        // タイムアウト発生
                        eprintln!("Warning: Progress update timed out");
                        break;
                    }
                }
            }
            
            // アップロード完了を待機
            upload_handle
                .await
                .context("Upload task panicked")?
                .context("Upload command failed")?
        }
        "help" => {
            commands::help::execute()
                .await
                .context("Help command failed")?
        }
        _ => bail!(
            "Unknown command: '{}'. Use 'help' to see available commands.",
            command
        ),
    };

    // コマンド結果を出力（プレゼンテーション層に委譲）
    output::output_result(&result, machine_output)?;

    Ok(())
}

/// コマンド使用方法を表示する
fn print_usage() {
    eprintln!("Usage: vidyeet [--machine] <command> [args...]");
    eprintln!();
    eprintln!("Global Flags:");
    eprintln!("  --machine        - Output machine-readable JSON to stdout (for scripting)");
    eprintln!();
    eprintln!("Available commands:");
    eprintln!("  login            - Login to Mux Video (credentials entered interactively)");
    eprintln!("  logout           - Logout from Mux Video");
    eprintln!("  status           - Check authentication status");
    eprintln!("  list             - List all uploaded videos");
    eprintln!("  upload <file>    - Upload a video to Mux Video");
    eprintln!("  help             - Display this help message");
}

/// 対話的に認証情報を取得
/// 
/// プレゼンテーション層の責務として、ユーザー入力を取得し検証する
fn read_credentials_interactive() -> Result<commands::login::LoginCredentials> {
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

    Ok(commands::login::LoginCredentials {
        token_id,
        token_secret,
    })
}

/// stdin からパイプで認証情報を取得（2行形式）
/// 
/// 形式:
///   1行目: Token ID
///   2行目: Token Secret
fn read_credentials_from_stdin() -> Result<commands::login::LoginCredentials> {
    let mut token_id = String::new();
    io::stdin()
        .read_line(&mut token_id)
        .context("Failed to read Token ID from stdin")?;
    let token_id = token_id.trim().to_string();

    if token_id.is_empty() {
        bail!("Token ID cannot be empty. Please ensure the first line of stdin contains a valid Token ID.");
    }

    let mut token_secret = String::new();
    io::stdin()
        .read_line(&mut token_secret)
        .context("Failed to read Token Secret from stdin")?;
    let token_secret = token_secret.trim().to_string();

    if token_secret.is_empty() {
        bail!("Token Secret cannot be empty. Please ensure the second line of stdin contains a valid Token Secret.");
    }

    Ok(commands::login::LoginCredentials {
        token_id,
        token_secret,
    })
}

/// アップロード進捗を人間向けに表示（stderr）
///
/// プレゼンテーション層の責務として、DisplayProgressを受け取り、
/// ユーザーフレンドリーなメッセージを表示します。
/// ドメイン層の実装詳細（UploadPhase）には依存しません。
fn display_upload_progress(progress: &DisplayProgress) {
    // Option<DisplayProgress>により表示すべき進捗のみ渡されるため、
    // 空文字チェック不要（呼び出し側でif let Some()によりフィルタ済み）
    eprintln!("{}", progress.message);
}

