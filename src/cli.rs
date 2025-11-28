use crate::commands::{self, CommandResult};
use anyhow::{Context, Result, bail};
use std::io::IsTerminal;

/// CLI引数を解析し、適切なコマンドにディスパッチする
pub async fn parse_args(args: &[String]) -> Result<()> {
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];

    let result = match command.as_str() {
        "login" => {
            commands::login::execute()
                .await
                .context("Login command failed")?
        }
        "logout" => {
            commands::logout::execute()
                .await
                .context("Logout command failed")?
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
    eprintln!("  upload <file>    - Upload a video to Mux Video");
    eprintln!("  help             - Display this help message");
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
            if r.was_logged_in {
                eprintln!("\n✓ Login credentials updated!");
                eprintln!("New authentication credentials have been saved.");
            } else {
                eprintln!("\n✓ Login successful!");
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
