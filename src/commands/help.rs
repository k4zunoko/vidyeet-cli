use crate::commands::result::CommandResult;

/// ヘルプコマンドを実行
///
/// # Returns
/// 成功時はOk(CommandResult)、失敗時はエラー
pub async fn execute() -> anyhow::Result<CommandResult> {
    Ok(CommandResult::Help)
}
