mod cli;
mod commands;
mod api;
mod config;
mod domain;

use std::env;
use anyhow::Result;
use domain::error::DomainError;
use api::error::InfraError;
use config::error::ConfigError;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if let Err(e) = run(&args) {
        handle_error(e);
    }
}

/// アプリケーションのメイン処理
fn run(args: &[String]) -> Result<()> {
    cli::parse_args(args)
}

/// エラーハンドリングとユーザーへの表示
/// 
/// anyhow::Error から元のエラー型を downcast して、
/// エラーの種類に応じた exit code とメッセージを決定する。
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
    
    // エラーの根本原因を downcast して判定
    let exit_code = determine_exit_code(&error);
    
    // ユーザー向けのヒントを表示
    if let Some(hint) = get_error_hint(&error) {
        eprintln!("\nHint: {}", hint);
    }
    
    // 適切な終了コードで終了
    std::process::exit(exit_code);
}

/// エラーチェーンから適切な終了コードを決定
fn determine_exit_code(error: &anyhow::Error) -> i32 {
    // エラーチェーン全体を探索
    for cause in error.chain() {
        // DomainError の場合
        if let Some(domain_err) = cause.downcast_ref::<DomainError>() {
            return domain_err.severity().exit_code();
        }
        
        // InfraError の場合
        if let Some(infra_err) = cause.downcast_ref::<InfraError>() {
            return infra_err.severity().exit_code();
        }
        
        // ConfigError の場合
        if let Some(config_err) = cause.downcast_ref::<ConfigError>() {
            return config_err.severity().exit_code();
        }
    }
    
    // 不明なエラーの場合はデフォルトの終了コード
    1
}

/// エラーに対するユーザー向けヒントを取得
fn get_error_hint(error: &anyhow::Error) -> Option<String> {
    for cause in error.chain() {
        // DomainError からヒントを取得
        if let Some(domain_err) = cause.downcast_ref::<DomainError>() {
            if let Some(hint) = domain_err.hint() {
                return Some(hint.to_string());
            }
        }
        
        // ConfigError からヒントを取得
        if let Some(config_err) = cause.downcast_ref::<ConfigError>() {
            if let Some(hint) = config_err.hint() {
                return Some(hint.to_string());
            }
        }
    }
    
    None
}
