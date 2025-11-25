/// 設定管理モジュール
///
/// このモジュールは2層の設定構造を提供します:
/// 1. AppConfig - ビルド時にコンパイル時定数として定義される静的設定（APP_CONFIG）
/// 2. UserConfig - 実行時に読み込まれる動的設定
///
/// # 使用例
///
/// ```rust
/// use crate::config::{APP_CONFIG, UserConfig};
///
/// // AppConfig: グローバル定数として直接参照
/// let endpoint = APP_CONFIG.api.endpoint;
/// let max_size = APP_CONFIG.upload.max_file_size;
///
/// // UserConfig: load時に自動検証
/// let user_config = UserConfig::load()?;
/// let api_key = user_config.api_key.as_ref().unwrap();
/// ```
pub mod app;
pub mod error;
pub mod user;

pub use app::APP_CONFIG;
pub use error::ConfigError;
pub use user::UserConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_direct_access() {
        // APP_CONFIGがグローバル定数として直接アクセス可能であることを確認
        assert_eq!(APP_CONFIG.api.endpoint, "https://ws.api.video");
        assert_eq!(APP_CONFIG.api.timeout_seconds, 30);
        assert!(!APP_CONFIG.upload.supported_formats.is_empty());
    }

    #[test]
    fn test_user_config_load_with_validation() {
        // UserConfig::load() が検証を含むことを確認
        // デフォルト設定ファイルを削除して、新規作成される状況をテスト
        let config_path = UserConfig::config_path().expect("Failed to get config path");
        if config_path.exists() {
            std::fs::remove_file(&config_path).ok();
        }

        // load() はデフォルト設定を作成するが、検証エラーになるはず（"your_api_key_here"）
        let result = UserConfig::load();
        assert!(result.is_err(), "Default config should fail validation");

        if let Err(crate::config::ConfigError::ValidationError { message }) = result {
            // エラーメッセージにデフォルトAPIキーの言及があることを確認
            assert!(
                message.contains("your_api_key_here") || message.contains("still the default"),
                "Expected validation error message about default API key, got: {}",
                message
            );
        } else {
            panic!("Expected ValidationError for default API key, got: {:?}", result);
        }
    }

    #[test]
    fn test_save_and_reload_with_valid_config() {
        // 有効な設定で保存・再読み込みのテスト
        let config_path = UserConfig::config_path().expect("Failed to get config path");
        if config_path.exists() {
            std::fs::remove_file(&config_path).ok();
        }

        // 有効なAPIキーで設定を作成
        let config = UserConfig {
            api_key: Some("valid_test_key_1234567890".to_string()),
            default_title: Some("Test Title".to_string()),
            auto_copy_url: true,
            show_notification: false,
        };

        // 検証が通ることを確認
        assert!(config.validate().is_ok());

        // 保存
        config.save().expect("Failed to save config");

        // 再読み込み（自動検証される）
        let reloaded = UserConfig::load().expect("Failed to reload config");
        assert_eq!(reloaded.api_key, config.api_key);
        assert_eq!(reloaded.default_title, config.default_title);
    }

    #[test]
    fn test_independent_config_usage() {
        // AppConfigとUserConfigが独立して使用できることを確認

        // AppConfig: 直接アクセス
        let max_size = APP_CONFIG.upload.max_file_size;
        assert!(max_size > 0);

        // UserConfig: 有効な設定を作成してテスト
        let user_config = UserConfig {
            api_key: Some("test_api_key_1234567890".to_string()),
            default_title: Some("My Video".to_string()),
            auto_copy_url: true,
            show_notification: true,
        };

        // 検証が通ることを確認
        assert!(user_config.validate().is_ok());
        assert!(user_config.has_api_key());
    }
}
