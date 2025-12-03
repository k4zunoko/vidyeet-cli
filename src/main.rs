mod api;
mod cli;
mod commands;
mod config;
mod domain;
mod error_severity;

use anyhow::Result;
use api::error::InfraError;
use config::error::ConfigError;
use config::user::UserConfig;
use domain::error::DomainError;
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if let Err(e) = run(&args).await {
        handle_error(e);
    }
}

/// アプリケーションのメイン処理
async fn run(args: &[String]) -> Result<()> {
    // アプリケーション起動時に設定ファイルが存在することを保証
    // 存在しない場合はデフォルト設定から自動生成される
    UserConfig::ensure_config_exists()?;

    cli::parse_args(args).await
}

/// エラーハンドリングとユーザーへの表示
///
/// エラーチェーンを一度走査して、最初にヒットしたアプリケーション定義エラーから
/// 終了コードとヒントを取得する。
fn handle_error(error: anyhow::Error) {
    // エラーメッセージのヘッダー
    eprintln!("Error: {}", error);

    // エラーチェーンを辿って詳細を表示
    let chain: Vec<_> = error.chain().skip(1).collect();
    if !chain.is_empty() {
        eprintln!("\nCaused by:");
        for (i, cause) in chain.iter().enumerate() {
            eprintln!("  {}: {}", i + 1, cause);
        }
    }

    // エラーチェーンから終了コードとヒントを同時取得
    let (exit_code, hint) = extract_error_info(&error);

    // ユーザー向けのヒントを表示
    if let Some(hint_text) = hint {
        eprintln!("\nHint: {}", hint_text);
    }

    // 適切な終了コードで終了
    std::process::exit(exit_code);
}

/// エラーチェーンから終了コードとヒントを一度の走査で抽出
///
/// 最初にヒットしたアプリケーション定義エラー（DomainError, ConfigError, InfraError）
/// から責務の委譲によりseverity() と hint() を取得する。
/// 型判定の重複を排除し、エラー型側への分類責務の委譲を実現。
fn extract_error_info(error: &anyhow::Error) -> (i32, Option<String>) {
    // エラーチェーン全体を一度走査
    for cause in error.chain() {
        // DomainError の場合
        if let Some(domain_err) = cause.downcast_ref::<DomainError>() {
            let severity = domain_err.severity();
            let hint = domain_err.hint().map(|s| s.to_string());
            return (severity.exit_code(), hint);
        }

        // ConfigError の場合
        if let Some(config_err) = cause.downcast_ref::<ConfigError>() {
            let severity = config_err.severity();
            let hint = config_err.hint().map(|s| s.to_string());
            return (severity.exit_code(), hint);
        }

        // InfraError の場合
        if let Some(infra_err) = cause.downcast_ref::<InfraError>() {
            let severity = infra_err.severity();
            return (severity.exit_code(), None);
        }
    }

    // 不明なエラーの場合はデフォルトの終了コード
    (1, None)
}
