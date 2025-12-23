use crate::commands;
use crate::presentation::input;
use crate::presentation::output;
use crate::presentation::progress;
use anyhow::{Context, Result, bail};

/// CLI引数を解析し、適切なコマンドにディスパッチする
pub async fn parse_args(args: &[String]) -> Result<()> {
    if args.len() < 2 {
        output::print_usage();
        return Ok(());
    }

    // グローバルフラグ --machine のチェック
    let (machine_output, command_start_index) = if args.len() > 1 && args[1] == "--machine" {
        (true, 2)
    } else {
        (false, 1)
    };

    if args.len() < command_start_index + 1 {
        output::print_usage();
        return Ok(());
    }

    let command = &args[command_start_index];

    let result = match command.as_str() {
        "login" => {
            // --stdin フラグをチェック
            let use_stdin =
                args.get(command_start_index + 1).map(|s| s.as_str()) == Some("--stdin");

            let credentials = if use_stdin {
                input::read_credentials_from_stdin()?
            } else {
                input::read_credentials_interactive()?
            };

            commands::login::execute(credentials)
                .await
                .context("Login command failed")?
        }
        "logout" => commands::logout::execute()
            .await
            .context("Logout command failed")?,
        "status" => commands::status::execute()
            .await
            .context("Status command failed")?,
        "list" => commands::list::execute()
            .await
            .context("List command failed")?,
        "show" => {
            let asset_id = args
                .get(command_start_index + 1)
                .context("Please specify an asset ID for show command")?;

            commands::show::execute(asset_id)
                .await
                .context("Show command failed")?
        }
        "delete" => {
            let asset_id = args
                .get(command_start_index + 1)
                .context("Please specify an asset ID for delete command")?
                .trim();

            if asset_id.is_empty() {
                bail!("Asset ID cannot be empty");
            }

            // --force フラグをチェック
            let force = args.get(command_start_index + 2).map(|s| s.as_str()) == Some("--force");

            // force フラグがない場合は確認プロンプトを表示
            if !force && !machine_output {
                let confirmed = input::confirm_delete(asset_id)?;
                if !confirmed {
                    // キャンセルされた場合は正常終了
                    return Ok(());
                }
            }

            commands::delete::execute(asset_id)
                .await
                .context("Delete command failed")?
        }
        "upload" => {
            let file_path = args
                .get(command_start_index + 1)
                .context("Please specify a file path for upload command")?
                .trim(); // 先頭・末尾の空白削除

            if file_path.is_empty() {
                bail!("File path cannot be empty");
            }

            // --progress フラグをチェック
            let show_progress =
                args.get(command_start_index + 2).map(|s| s.as_str()) == Some("--progress");

            // 進捗通知チャネルを作成
            let (progress_tx, progress_rx) = tokio::sync::mpsc::channel(32);

            // アップロード処理を別タスクで開始
            let upload_handle = tokio::spawn({
                let file_path = file_path.to_string();
                async move { commands::upload::execute(&file_path, Some(progress_tx)).await }
            });

            // 進捗受信ループ（プレゼンテーション層に委譲）
            let progress_handle = tokio::spawn(async move {
                progress::handle_upload_progress(progress_rx, machine_output, show_progress).await
            });

            // 両方のタスクの完了を待機
            let upload_result = upload_handle
                .await
                .context("Upload task panicked")?
                .context("Upload command failed")?;

            progress_handle
                .await
                .context("Progress handler panicked")?
                .context("Progress handler failed")?;

            upload_result
        }
        "help" => commands::help::execute()
            .await
            .context("Help command failed")?,
        _ => bail!(
            "Unknown command: '{}'. Use 'help' to see available commands.",
            command
        ),
    };

    // コマンド結果を出力（プレゼンテーション層に委譲）
    output::output_result(&result, machine_output)?;

    Ok(())
}
