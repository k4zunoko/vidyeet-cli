use crate::commands;

/// CLI引数を解析し、適切なコマンドにディスパッチする
pub fn parse_args(args: &[String]) -> Result<(), String> {
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];

    match command.as_str() {
        "upload" => {
            let file_path = args.get(2)
                .ok_or("Error: Please specify a file path")?;
            commands::upload::execute(file_path)
        }
        "help" => {
            commands::help::execute();
            Ok(())
        }
        _ => Err(format!("Unknown command: {}", command))
    }
}

/// コマンド使用方法を表示する
fn print_usage() {
    println!("Usage: streamable-cli <command> [args...]");
    println!("Available commands:");
    println!("  upload <file>  - Upload a video to Streamable");
    println!("  help           - Display this help message");
}
