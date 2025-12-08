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
/// let refresh_token = user_config.get_refresh_token()?;
/// ```
pub mod app;
pub mod error;
pub mod user;

pub use app::{APP_CONFIG, BYTES_PER_MB};
pub use user::UserConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_direct_access() {
        // APP_CONFIGがグローバル定数として直接アクセス可能であることを確認
        assert_eq!(APP_CONFIG.api.endpoint, "https://api.mux.com");
        assert_eq!(APP_CONFIG.api.timeout_seconds, 300);
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

        // load() はデフォルト設定を作成する（認証なしでも成功）
        let result = UserConfig::load();
        assert!(result.is_ok(), "Default config should load successfully");

        let config = result.unwrap();
        assert!(!config.has_auth(), "Default config should not have auth");
    }

    #[test]
    fn test_save_and_reload_with_valid_config() {
        // 有効な設定で保存・再読み込みのテスト
        let config_path = UserConfig::config_path().expect("Failed to get config path");
        if config_path.exists() {
            std::fs::remove_file(&config_path).ok();
        }

        // 認証情報で設定を作成
        let mut config = UserConfig {
            auth: None,
            timezone_offset_seconds: 0, // UTC
        };
        config.set_auth("test_id".to_string(), "test_secret".to_string());

        // 検証が通ることを確認
        assert!(config.validate().is_ok());

        // 保存
        config.save().expect("Failed to save config");

        // 再読み込み（自動検証される）
        let reloaded = UserConfig::load().expect("Failed to reload config");
        let reloaded_auth = reloaded.get_auth().expect("Auth should be present");
        let config_auth = config.get_auth().expect("Auth should be present");
        assert_eq!(reloaded_auth.token_id, config_auth.token_id);
        assert_eq!(reloaded_auth.token_secret, config_auth.token_secret);
        assert_eq!(reloaded.timezone_offset_seconds, config.timezone_offset_seconds);
    }

    #[test]
    fn test_independent_config_usage() {
        // AppConfigとUserConfigが独立して使用できることを確認

        // AppConfig: 直接アクセス
        let max_size = APP_CONFIG.upload.max_file_size;
        assert!(max_size > 0);

        // UserConfig: 有効な設定を作成してテスト
        let mut user_config = UserConfig {
            auth: None,
            timezone_offset_seconds: 0, // UTC
        };
        user_config.set_auth("test_id".to_string(), "test_secret".to_string());

        // 検証が通ることを確認
        assert!(user_config.validate().is_ok());
        assert!(user_config.has_auth());
    }
}
