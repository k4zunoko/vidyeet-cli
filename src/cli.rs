use crate::commands;
use anyhow::{Context, Result, bail};

/// CLI引数を解析し、適切なコマンドにディスパッチする
pub async fn parse_args(args: &[String]) -> Result<()> {
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];

    match command.as_str() {
        "login" => {
            commands::login::execute()
                .await
                .context("Login command failed")
        }
        "logout" => {
            commands::logout::execute()
                .await
                .context("Logout command failed")
        }
        "upload" => {
            let file_path = args
                .get(2)
                .context("Please specify a file path for upload command")?;
            commands::upload::execute(file_path)
                .await
                .context("Upload command failed")
        }
        "help" => {
            commands::help::execute();
            Ok(())
        }
        _ => bail!(
            "Unknown command: '{}'. Use 'help' to see available commands.",
            command
        ),
    }
}

/// コマンド使用方法を表示する
fn print_usage() {
    println!("Usage: vidyeet <command> [args...]");
    println!("Available commands:");
    println!("  login            - Login to Mux Video (credentials entered interactively)");
    println!("  logout           - Logout from Mux Video");
    println!("  upload <file>    - Upload a video to Mux Video");
    println!("  help             - Display this help message");
}
