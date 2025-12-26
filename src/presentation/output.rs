/// プレゼンテーション層: コマンド結果の出力
///
/// コマンド実行結果をユーザー向け（人間可読）または
/// 機械向け（JSON）形式で出力する責務を担います。
/// CLI使用方法の表示もこのモジュールが担当します。
use crate::commands::result::{CommandResult, Mp4Status};
use anyhow::Result;

/// ヘルプテキスト（単一の情報源）
const HELP_TEXT: &str = "vidyeet-CLI
Upload videos to Mux Video easily from the command line

Usage:
  vidyeet [--machine] <command> [args...]

Global Flags:
  --machine        - Output machine-readable JSON to stdout (for scripting)
                     Works for both success and error cases

Available commands:
  login [--stdin]  - Login to Mux Video
                     Without --stdin: Interactive credential input (default)
                     With --stdin: Read credentials from standard input
                                   Format: line 1 = Token ID, line 2 = Token Secret
  logout           - Logout from Mux Video
  status           - Check authentication status
  list             - List all uploaded videos
  show <asset_id>  - Show detailed information about a specific video asset
  delete <asset_id> [--force]
                   - Delete a video asset from Mux Video
                     --force: Skip confirmation prompt
  upload <file> [--progress]
                   - Upload a video to Mux Video
                     --progress: Show upload progress (required for progress output)
  help             - Display this help message

Machine-Readable Output:
  --machine status               - JSON output for success
  --machine list                 - JSON output with error handling
  echo \"id\nkey\" | --machine login --stdin
                                 - Automated login with JSON response

Error Output:
  Normal mode:   Human-readable error messages to stderr
  --machine:     JSON error object with exit_code and hint fields

Progress Output:
  upload --progress              - Show human-readable progress to stderr
  --machine upload --progress    - Output machine-readable JSON progress to stdout";

/// コマンド使用方法を表示する
///
/// CLI引数が不正な場合や、ヘルプが必要な場合に呼び出されます。
pub fn print_usage() {
    eprintln!("{}", HELP_TEXT);
}

/// コマンド結果を適切な形式で出力する
///
/// # Arguments
/// * `result` - コマンド実行結果
/// * `machine_output` - 機械可読出力フラグ
///
/// # Output
/// * `machine_output = false`: 人間向けの詳細メッセージ（stderr）
/// * `machine_output = true`: 機械可読JSON（stdout）
pub fn output_result(result: &CommandResult, machine_output: bool) -> Result<()> {
    if machine_output {
        output_machine_readable(result)?;
    } else {
        output_human_readable(result)?;
    }

    Ok(())
}

/// 人間向けの詳細メッセージを出力（stderr）
///
/// ユーザーが理解しやすい形式でコマンド結果を表示します。
/// すべての出力はstderrに送られ、stdoutはパイプライン用に予約されます。
fn output_human_readable(result: &CommandResult) -> Result<()> {
    match result {
        CommandResult::Login(r) => {
            eprintln!();
            if r.was_logged_in {
                eprintln!("✓ Login credentials updated!");
                eprintln!("New authentication credentials have been saved.");
            } else {
                eprintln!("Login successful.");
                eprintln!("Authentication credentials have been saved.");
            }
        }
        CommandResult::Logout(r) => {
            if r.was_logged_in {
                eprintln!("Logged out successfully.");
                eprintln!("Authentication credentials have been removed.");
            } else {
                eprintln!("Already logged out.");
            }
        }
        CommandResult::Status(r) => {
            eprintln!();
            if r.is_authenticated {
                eprintln!("Authenticated");
                if let Some(token_id) = &r.token_id {
                    eprintln!("Token ID: {}", token_id);
                }
                eprintln!();
                eprintln!("Your credentials are valid and working.");
            } else if let Some(token_id) = &r.token_id {
                // 認証情報はあるが検証失敗
                eprintln!("✗ Authentication failed");
                eprintln!("  Token ID: {}", token_id);
                eprintln!();
                eprintln!("Your credentials may be invalid or expired.");
                eprintln!("Please run 'vidyeet login' to update your credentials.");
            } else {
                // 認証情報が存在しない
                eprintln!("Not logged in");
                eprintln!("No authentication credentials found.");
                eprintln!("Please run 'vidyeet login' to authenticate.");
            }
        }
        CommandResult::List(r) => {
            eprintln!();
            if r.total_count == 0 {
                eprintln!("No videos found.");
                eprintln!("Upload your first video with 'vidyeet upload <file>'");
            } else {
                // ユーザー設定を読み込んでタイムゾーン設定を取得
                let user_config = crate::config::user::UserConfig::load().ok();

                eprintln!("Found {} video(s):", r.total_count);
                eprintln!();
                for (idx, video) in r.videos.iter().enumerate() {
                    eprintln!("---");
                    eprintln!("Video #{}", idx + 1);
                    eprintln!("Asset ID: {}", video.asset_id);
                    eprintln!("Status: {}", video.status);

                    if let Some(duration) = video.duration {
                        let minutes = (duration / 60.0) as u64;
                        let seconds = (duration % 60.0) as u64;
                        eprintln!("Duration: {}:{:02}", minutes, seconds);
                    }

                    if let Some(aspect_ratio) = &video.aspect_ratio {
                        eprintln!("Aspect Ratio: {}", aspect_ratio);
                    }

                    if let Some(hls_url) = &video.hls_url {
                        eprintln!("HLS URL: {}", hls_url);
                    }
                    if let Some(mp4_url) = &video.mp4_url {
                        eprintln!("MP4 URL: {}", mp4_url);
                    }

                    // 作成日時をフォーマット（ユーザー設定のタイムゾーンを使用）
                    let formatted_time = if let Some(config) = &user_config {
                        crate::domain::formatter::format_timestamp(&video.created_at, config)
                    } else {
                        video.created_at.clone()
                    };
                    eprintln!("Created: {}", formatted_time);
                    eprintln!();
                }
                eprintln!("---");
            }
        }
        CommandResult::Show(r) => {
            eprintln!();
            eprintln!("Asset Details:");
            eprintln!("==============");
            eprintln!("Asset ID:       {}", r.asset_id);
            eprintln!("Status:         {}", r.status);

            if let Some(duration) = r.duration {
                let minutes = (duration / 60.0) as u64;
                let seconds = (duration % 60.0) as u64;
                eprintln!(
                    "Duration:       {}:{:02} ({:.2}s)",
                    minutes, seconds, duration
                );
            }

            if let Some(aspect_ratio) = &r.aspect_ratio {
                eprintln!("Aspect Ratio:   {}", aspect_ratio);
            }

            if let Some(video_quality) = &r.video_quality {
                eprintln!("Video Quality:  {}", video_quality);
            }

            // 作成日時をフォーマット（ユーザー設定のタイムゾーンを使用）
            let user_config = crate::config::user::UserConfig::load().ok();
            let formatted_time = if let Some(config) = &user_config {
                crate::domain::formatter::format_timestamp(&r.created_at, config)
            } else {
                r.created_at.clone()
            };
            eprintln!("Created At:     {}", formatted_time);

            eprintln!();
            eprintln!("Playback Information:");
            eprintln!("--------------------");

            if !r.playback_ids.is_empty() {
                for (idx, playback_id) in r.playback_ids.iter().enumerate() {
                    eprintln!("Playback ID #{}: {}", idx + 1, playback_id.id);
                    eprintln!("  Policy:       {}", playback_id.policy);
                }
            } else {
                eprintln!("No playback IDs available");
            }

            if let Some(hls_url) = &r.hls_url {
                eprintln!("HLS URL:        {}", hls_url);
            }

            if let Some(mp4_url) = &r.mp4_url {
                eprintln!("MP4 URL:        {}", mp4_url);
            }

            if let Some(tracks) = &r.tracks
                && !tracks.is_empty()
            {
                eprintln!();
                eprintln!("Tracks:");
                eprintln!("-------");
                for (idx, track) in tracks.iter().enumerate() {
                    eprint!("Track #{}: {} ", idx + 1, track.track_type);
                    if let Some(duration) = track.duration {
                        eprint!("(duration: {:.2}s)", duration);
                    }
                    eprintln!();
                }
            }

            if let Some(renditions) = &r.static_renditions
                && !renditions.files.is_empty()
            {
                eprintln!();
                eprintln!("Static Renditions:");
                eprintln!("------------------");
                for (idx, rendition) in renditions.files.iter().enumerate() {
                    eprintln!("Rendition #{}: {}", idx + 1, rendition.name);
                    eprintln!("  Status:       {}", rendition.status);
                    eprintln!("  Resolution:   {}", rendition.resolution);
                    eprintln!("  Type:         {}", rendition.rendition_type);
                    eprintln!("  Format:       {}", rendition.ext);
                }
            }
            eprintln!();
        }
        CommandResult::Upload(r) => {
            eprintln!("\nUpload completed successfully!");
            eprintln!("---");
            eprintln!("Asset ID: {}", r.asset_id);

            // HLS再生URL（すぐに利用可能）
            if let Some(hls_url) = &r.hls_url {
                eprintln!("\nHLS Streaming URL:");
                eprintln!("{}", hls_url);
            }

            // MP4再生URL（アプリケーション層で既に生成済み）
            eprintln!("\nMP4 Download URL:");
            if let Some(mp4_url) = &r.mp4_url {
                eprintln!("{}", mp4_url);

                // MP4生成中の場合のみ注記を表示
                if matches!(r.mp4_status, Mp4Status::Generating) {
                    eprintln!(
                        "\nNote: MP4 file is being generated in the background (usually 2-5 minutes)."
                    );
                    eprintln!("The URL above will be available once generation completes.");
                    eprintln!("You can start streaming with HLS URL immediately!");
                }
            } else {
                eprintln!("(not available)");
            }

            eprintln!("---");

            // 削除した動画がある場合
            if r.deleted_old_videos > 0 {
                eprintln!(
                    "\nNote: Deleted {} old video(s) because the video limit for your plan was reached.",
                    r.deleted_old_videos
                );
            }
        }
        CommandResult::Delete(r) => {
            eprintln!();
            eprintln!("✓ Asset deleted successfully!");
            eprintln!("Asset ID: {}", r.asset_id);
            eprintln!();
            eprintln!("The video and all its data have been permanently removed.");
        }
        CommandResult::Help => {
            eprintln!("{}", HELP_TEXT);
        }
    }

    Ok(())
}

/// 機械可読JSONを出力（stdout）
///
/// スクリプトやパイプライン処理のために、
/// コマンド結果を構造化されたJSON形式で出力します。
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
            // raw_assetsがある場合（--machine フラグ時）は完全データを出力
            if let Some(raw_assets) = &r.raw_assets {
                serde_json::json!({
                    "success": true,
                    "command": "list",
                    "data": raw_assets,
                    "total_count": r.total_count
                })
            } else {
                // 簡略版を出力（人間向けの互換性維持）
                serde_json::json!({
                    "success": true,
                    "command": "list",
                    "videos": r.videos,
                    "total_count": r.total_count
                })
            }
        }
        CommandResult::Show(r) => {
            // raw_assetがある場合は完全データを出力
            if let Some(raw_asset) = &r.raw_asset {
                serde_json::json!({
                    "success": true,
                    "command": "show",
                    "data": raw_asset
                })
            } else {
                // 簡略版を出力（互換性維持）
                serde_json::json!({
                    "success": true,
                    "command": "show",
                    "asset_id": r.asset_id,
                    "status": r.status,
                    "duration": r.duration,
                    "aspect_ratio": r.aspect_ratio,
                    "video_quality": r.video_quality,
                    "created_at": r.created_at,
                    "playback_ids": r.playback_ids,
                    "hls_url": r.hls_url,
                    "mp4_url": r.mp4_url,
                    "tracks": r.tracks,
                    "static_renditions": r.static_renditions
                })
            }
        }
        CommandResult::Upload(r) => {
            serde_json::json!({
                "success": true,
                "command": "upload",
                "asset_id": r.asset_id,
                "playback_id": r.playback_id,
                "hls_url": r.hls_url,
                "mp4_url": r.mp4_url,
                "mp4_status": r.mp4_status,
                "file_path": r.file_path,
                "file_size": r.file_size,
                "file_format": r.file_format,
                "deleted_old_videos": r.deleted_old_videos
            })
        }
        CommandResult::Delete(r) => {
            serde_json::json!({
                "success": true,
                "command": "delete",
                "asset_id": r.asset_id
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::result::{
        ListResult, LoginResult, LogoutResult, Mp4Status, StatusResult, UploadResult,
    };

    #[test]
    fn test_output_machine_readable_login() {
        let result = CommandResult::Login(LoginResult {
            was_logged_in: false,
        });

        // JSON出力が正しく生成されることを確認
        let output = output_machine_readable(&result);
        assert!(output.is_ok());
    }

    #[test]
    fn test_output_machine_readable_logout() {
        let result = CommandResult::Logout(LogoutResult {
            was_logged_in: true,
        });

        let output = output_machine_readable(&result);
        assert!(output.is_ok());
    }

    #[test]
    fn test_output_machine_readable_status_authenticated() {
        let result = CommandResult::Status(StatusResult {
            is_authenticated: true,
            token_id: Some("test_token_masked".to_string()),
        });

        let output = output_machine_readable(&result);
        assert!(output.is_ok());
    }

    #[test]
    fn test_output_machine_readable_list_empty() {
        let result = CommandResult::List(ListResult {
            videos: vec![],
            total_count: 0,
            raw_assets: None,
        });

        let output = output_machine_readable(&result);
        assert!(output.is_ok());
    }

    #[test]
    fn test_output_machine_readable_upload() {
        let result = CommandResult::Upload(UploadResult {
            asset_id: "test_asset_123".to_string(),
            playback_id: Some("test_playback_123".to_string()),
            hls_url: Some("https://stream.mux.com/test.m3u8".to_string()),
            mp4_url: Some("https://stream.mux.com/test/highest.mp4".to_string()),
            mp4_status: Mp4Status::Ready,
            file_path: "/path/to/video.mp4".to_string(),
            file_size: 10485760,
            file_format: "mp4".to_string(),
            deleted_old_videos: 0,
        });

        let output = output_machine_readable(&result);
        assert!(output.is_ok());
    }

    #[test]
    fn test_output_machine_readable_help() {
        let result = CommandResult::Help;

        let output = output_machine_readable(&result);
        assert!(output.is_ok());
    }

    #[test]
    fn test_output_human_readable_login() {
        let result = CommandResult::Login(LoginResult {
            was_logged_in: false,
        });

        // 人間向け出力がエラーなく実行されることを確認
        let output = output_human_readable(&result);
        assert!(output.is_ok());
    }

    #[test]
    fn test_output_result_machine_mode() {
        let result = CommandResult::Help;

        // --machine フラグでJSON出力
        let output = output_result(&result, true);
        assert!(output.is_ok());
    }

    #[test]
    fn test_output_result_human_mode() {
        let result = CommandResult::Help;

        // 通常モードで人間向け出力
        let output = output_result(&result, false);
        assert!(output.is_ok());
    }
}
