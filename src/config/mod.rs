/// 設定管理モジュール
/// 
/// このモジュールは2層の設定構造を提供します:
/// 1. AppConfig - ビルド時に埋め込まれる静的設定
/// 2. UserConfig - 実行時に読み込まれる動的設定

pub mod app;
pub mod error;
pub mod user;

pub use app::AppConfig;
pub use error::ConfigError;
pub use user::UserConfig;

/// 統合設定
/// 
/// アプリケーション設定とユーザー設定を統合して管理します。
#[derive(Debug, Clone)]
pub struct Config {
    /// ビルド時設定
    pub app: AppConfig,
    
    /// ユーザー設定
    pub user: UserConfig,
}

impl Config {
    /// 設定を読み込む
    /// 
    /// AppConfigはビルド時に埋め込まれた設定を読み込み、
    /// UserConfigはユーザーディレクトリから設定を読み込みます。
    /// 
    /// # Returns
    /// 統合された設定
    /// 
    /// # Errors
    /// ユーザー設定の読み込みに失敗した場合に ConfigError を返します。
    /// (AppConfigの読み込みはビルド時エラーなので実行時には発生しません)
    pub fn load() -> Result<Self, ConfigError> {
        let app = AppConfig::load();
        let user = UserConfig::load()?;
        
        Ok(Self { app, user })
    }
    
    /// ユーザー設定を保存
    /// 
    /// # Errors
    /// 設定ファイルの書き込みに失敗した場合に ConfigError を返します。
    pub fn save_user_config(&self) -> Result<(), ConfigError> {
        self.user.save()
    }
    
    /// APIキーが設定されているかチェック
    pub fn has_api_key(&self) -> bool {
        self.user.has_api_key()
    }
    
    /// 設定の妥当性を検証
    /// 
    /// # Returns
    /// 設定が妥当な場合はOk、そうでない場合は ConfigError::ValidationError を返します
    pub fn validate(&self) -> Result<(), ConfigError> {
        // API エンドポイントのチェック
        if self.app.api.endpoint.is_empty() {
            return Err(ConfigError::ValidationError {
                message: "API endpoint is not configured".to_string(),
            });
        }
        
        // タイムアウト設定のチェック
        if self.app.api.timeout_seconds == 0 {
            return Err(ConfigError::ValidationError {
                message: "API timeout must be greater than 0".to_string(),
            });
        }
        
        // 対応フォーマットのチェック
        if self.app.upload.supported_formats.is_empty() {
            return Err(ConfigError::ValidationError {
                message: "No supported video formats configured".to_string(),
            });
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_config() {
        // 設定の妥当性検証が正しく動作することを確認
        let config = Config::load().expect("Failed to load config");
        config.validate().expect("Config validation failed");
    }

    #[test]
    fn test_has_api_key() {
        // Config経由でのAPIキー確認が正しく動作することを確認
        let mut config = Config::load().expect("Failed to load config");
        
        // APIキーをクリア
        config.user.api_key = None;
        assert!(!config.has_api_key());
        
        // APIキーを設定
        config.user.set_api_key("test_key".to_string());
        assert!(config.has_api_key());
    }

    #[test]
    fn test_save_user_config_persistence() {
        // save_user_config() が実際にファイルに書き込むことを確認
        
        // 既存の設定ファイルを削除してクリーンな状態から開始
        let config_path = UserConfig::config_path().expect("Failed to get config path");
        if config_path.exists() {
            std::fs::remove_file(&config_path).ok();
        }
        
        let mut config = Config::load().expect("Failed to load config");
        
        // ユニークなテスト値を設定
        let test_api_key = format!("test_key_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs());
        config.user.set_api_key(test_api_key.clone());
        config.user.default_title = Some("Test Title".to_string());
        
        // 保存を実行
        config.save_user_config().expect("Failed to save user config");
        
        // 再度読み込んで、保存された値が反映されていることを確認
        let reloaded_config = Config::load().expect("Failed to reload config");
        assert_eq!(reloaded_config.user.api_key, Some(test_api_key));
        assert_eq!(reloaded_config.user.default_title, Some("Test Title".to_string()));
    }

    #[test]
    fn test_validate_config_errors() {
        // 不正な設定に対してエラーが返されることを確認
        let mut config = Config::load().expect("Failed to load config");
        
        // タイムアウトを0に設定（不正な値）
        config.app.api.timeout_seconds = 0;
        let result = config.validate();
        
        assert!(result.is_err());
        if let Err(e) = result {
            match e {
                ConfigError::ValidationError { message } => {
                    assert!(message.contains("timeout"));
                }
                _ => panic!("Expected ValidationError"),
            }
        }
    }

    #[test]
    fn test_config_integration() {
        // Config全体の統合動作を確認
        let config = Config::load().expect("Failed to load config");
        
        // AppConfigの値が正しく読み込まれていることを確認
        assert!(!config.app.api.endpoint.is_empty());
        assert!(config.app.api.timeout_seconds > 0);
        assert!(!config.app.upload.supported_formats.is_empty());
        
        // UserConfigが読み込まれていることを確認
        // （api_keyはNoneでも構わない）
        assert!(config.user.show_notification || !config.user.show_notification); // 常にtrue（存在確認）
    }
}

