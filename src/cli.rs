use crate::commands::{self, CommandResult};
use anyhow::{Context, Result, bail};
use std::io::{self, IsTerminal, Write};

/// CLI引数を解析し、適切なコマンドにディスパッチする
pub async fn parse_args(args: &[String]) -> Result<()> {
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];

    let result = match command.as_str() {
        "login" => {
            // --stdin フラグをチェック
            let use_stdin = args.get(2).map(|s| s.as_str()) == Some("--stdin");
            
            let credentials = if use_stdin {
                // stdin からパイプで認証情報を取得
                read_credentials_from_stdin()
                    .context("Failed to read credentials from stdin")?
            } else {
                // 対話的入力の場合
                if !io::stdin().is_terminal() {
                    bail!("Interactive input requires a TTY. Use '--stdin' flag for non-interactive input.");
                }
                
                // 案内メッセージを表示（プレゼンテーション層の責務）
                eprintln!("Logging in to Mux Video...");
                eprintln!();
                eprintln!("Please enter your Mux Access Token credentials.");
                eprintln!("You can find them at: https://dashboard.mux.com/settings/access-tokens");
                eprintln!();
                
                // 既存ログイン時の警告チェック
                if let Ok(config) = crate::config::user::UserConfig::load() {
                    if config.has_auth() {
                        eprintln!("Note: You are already logged in. Entering new credentials will overwrite the existing ones.");
                        eprintln!();
                    }
                }
                
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
            eprintln!("Checking authentication status...");
            eprintln!();
            
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
                .get(2)
                .context("Please specify a file path for upload command")?;
            commands::upload::execute(file_path)
                .await
                .context("Upload command failed")?
        }
        "help" => {
            commands::help::execute();
            return Ok(());
        }
        _ => bail!(
            "Unknown command: '{}'. Use 'help' to see available commands.",
            command
        ),
    };

    // コマンド結果を出力
    output_result(&result)?;

    Ok(())
}

/// コマンド使用方法を表示する
fn print_usage() {
    eprintln!("Usage: vidyeet <command> [args...]");
    eprintln!("Available commands:");
    eprintln!("  login            - Login to Mux Video (credentials entered interactively)");
    eprintln!("  logout           - Logout from Mux Video");
    eprintln!("  status           - Check authentication status");
    eprintln!("  list             - List all uploaded videos");
    eprintln!("  upload <file>    - Upload a video to Mux Video");
    eprintln!("  help             - Display this help message");
}

/// 対話的に認証情報を取得（TTY必須）
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

    Ok(commands::login::LoginCredentials {
        token_id,
        token_secret,
    })
}

/// コマンド結果を適切な形式で出力する
/// 
/// TTY接続時: 人間向けの詳細メッセージ（stderr）
/// パイプ/リダイレクト時: 機械可読JSON（stdout）
fn output_result(result: &CommandResult) -> Result<()> {
    let is_terminal = std::io::stdout().is_terminal();

    if is_terminal {
        // 人間向け出力（stderr）
        output_human_readable(result)?;
    } else {
        // 機械可読出力（stdout）
        output_machine_readable(result)?;
    }

    Ok(())
}

/// 人間向けの詳細メッセージを出力（stderr）
fn output_human_readable(result: &CommandResult) -> Result<()> {
    match result {
        CommandResult::Login(r) => {
            eprintln!();
            if r.was_logged_in {
                eprintln!("✓ Login credentials updated!");
                eprintln!("New authentication credentials have been saved.");
            } else {
                eprintln!("✓ Login successful!");
                eprintln!("Authentication credentials have been saved.");
            }
        }
        CommandResult::Logout(r) => {
            if r.was_logged_in {
                eprintln!("✓ Logged out successfully.");
                eprintln!("Authentication credentials have been removed.");
            } else {
                eprintln!("Already logged out.");
            }
        }
        CommandResult::Status(r) => {
            eprintln!();
            if r.is_authenticated {
                eprintln!("✓ Authenticated");
                if let Some(token_id) = &r.token_id {
                    eprintln!("  Token ID: {}", token_id);
                }
                eprintln!();
                eprintln!("  Your credentials are valid and working.");
            } else if let Some(token_id) = &r.token_id {
                // 認証情報はあるが検証失敗
                eprintln!("✗ Authentication failed");
                eprintln!("  Token ID: {}", token_id);
                eprintln!();
                eprintln!("  Your credentials may be invalid or expired.");
                eprintln!("  Please run 'vidyeet login' to update your credentials.");
            } else {
                // 認証情報が存在しない
                eprintln!("✗ Not logged in");
                eprintln!("  No authentication credentials found.");
                eprintln!("  Please run 'vidyeet login' to authenticate.");
            }
        }
        CommandResult::List(r) => {
            eprintln!();
            if r.total_count == 0 {
                eprintln!("No videos found.");
                eprintln!("Upload your first video with 'vidyeet upload <file>'");
            } else {
                eprintln!("✓ Found {} video(s):", r.total_count);
                eprintln!();
                for (idx, video) in r.videos.iter().enumerate() {
                    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    eprintln!("  Video #{}", idx + 1);
                    eprintln!("  Asset ID: {}", video.asset_id);
                    eprintln!("  Status: {}", video.status);
                    
                    if let Some(duration) = video.duration {
                        let minutes = (duration / 60.0) as u64;
                        let seconds = (duration % 60.0) as u64;
                        eprintln!("  Duration: {}:{:02}", minutes, seconds);
                    }
                    
                    if let Some(aspect_ratio) = &video.aspect_ratio {
                        eprintln!("  Aspect Ratio: {}", aspect_ratio);
                    }
                    
                    if let Some(hls_url) = &video.hls_url {
                        eprintln!("  🎬 HLS URL: {}", hls_url);
                    }
                    if let Some(mp4_url) = &video.mp4_url {
                        eprintln!("  📦 MP4 URL: {}", mp4_url);
                    }
                    
                    eprintln!("  Created: {}", video.created_at);
                    eprintln!();
                }
                eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            }
        }
        CommandResult::Upload(r) => {
            eprintln!("\n✓ Upload completed successfully!");
            eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            eprintln!("  Asset ID: {}", r.asset_id);
            
            // HLS再生URL（すぐに利用可能）
            if let Some(hls_url) = &r.hls_url {
                eprintln!("\n  🎬 HLS Streaming URL (ready now):");
                eprintln!("     {}", hls_url);
            }
            
            // MP4再生URL
            eprintln!("\n  📦 MP4 Download URL:");
            if let Some(mp4_url) = &r.mp4_url {
                eprintln!("     Status: ✓ Ready");
                eprintln!("     {}", mp4_url);
            } else {
                // MP4生成中の場合、予測URLを表示（playback_idベース）
                let predicted_url = if let Some(pid) = &r.playback_id {
                    format!("https://stream.mux.com/{}/highest.mp4", pid)
                } else {
                    // playback_idが未取得の場合は予測不能。案内のみ。
                    String::from("(playback_id not available yet)")
                };
                eprintln!("     Status: ⏳ Generating...");
                eprintln!("     {}", predicted_url);
                eprintln!("\n     Note: MP4 file is being generated in the background (usually 2-5 minutes).");
                eprintln!("           The URL above will be available once generation completes.");
                eprintln!("           You can start streaming with HLS URL immediately!");
            }
            
            eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

            // 削除した動画がある場合
            if r.deleted_old_videos > 0 {
                eprintln!("\nNote: Deleted {} old video(s) to stay within the 10-video limit.", 
                    r.deleted_old_videos);
            }
        }
        CommandResult::Help => {
            // Help コマンドは既に出力済み
        }
    }

    Ok(())
}

/// 機械可読JSONを出力（stdout）
fn output_machine_readable(result: &CommandResult) -> Result<()> {
    let json = match result {
        CommandResult::Login(r) => {
            serde_json::json!({
                "success": true,
                "command": "login",
                "was_logged_in": r.was_logged_in,
                "action": if r.was_logged_in { "updated" } else { "created" }
            })
        }
        CommandResult::Logout(r) => {
            serde_json::json!({
                "success": true,
                "command": "logout",
                "was_logged_in": r.was_logged_in
            })
        }
        CommandResult::Status(r) => {
            serde_json::json!({
                "success": true,
                "command": "status",
                "is_authenticated": r.is_authenticated,
                "token_id": r.token_id
            })
        }
        CommandResult::List(r) => {
            serde_json::json!({
                "success": true,
                "command": "list",
                "videos": r.videos,
                "total_count": r.total_count
            })
        }
        CommandResult::Upload(r) => {
            // MP4 URLが取得できない場合、予想URLを生成
            let mp4_url = r.mp4_url.clone().unwrap_or_else(|| {
                if let Some(pid) = &r.playback_id {
                    format!("https://stream.mux.com/{}/highest.mp4", pid)
                } else {
                    String::from("")
                }
            });
            
            serde_json::json!({
                "success": true,
                "command": "upload",
                "asset_id": r.asset_id,
                "playback_id": r.playback_id,
                "hls_url": r.hls_url,
                "mp4_url": mp4_url,
                "mp4_status": r.mp4_status,
                "file_path": r.file_path,
                "file_size": r.file_size,
                "file_format": r.file_format,
                "deleted_old_videos": r.deleted_old_videos
            })
        }
        CommandResult::Help => {
            serde_json::json!({
                "success": true,
                "command": "help"
            })
        }
    };

    println!("{}", serde_json::to_string(&json)?);
    Ok(())
}
