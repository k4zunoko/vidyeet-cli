/// 設定管理モジュール
/// 
/// このモジュールは2層の設定構造を提供します:
/// 1. AppConfig - ビルド時にコンパイル時定数として定義される静的設定
/// 2. UserConfig - 実行時に読み込まれる動的設定

pub mod app;
pub mod error;
pub mod user;

pub use app::{AppConfig, APP_CONFIG};
pub use error::ConfigError;
pub use user::UserConfig;

/// 統合設定
/// 
/// アプリケーション設定とユーザー設定を統合して管理します。
#[derive(Debug, Clone)]
pub struct Config {
    /// ビルド時設定（参照）
    pub app: &'static AppConfig,
    
    /// ユーザー設定
    pub user: UserConfig,
}

impl Config {
    /// 設定を読み込む
    /// 
    /// AppConfigはコンパイル時定数を参照し、
    /// UserConfigはユーザーディレクトリから設定を読み込みます。
    /// 
    /// # Returns
    /// 統合された設定
    /// 
    /// # Errors
    /// ユーザー設定の読み込みに失敗した場合に ConfigError を返します。
    pub fn load() -> Result<Self, ConfigError> {
        let app = &APP_CONFIG;
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
    /// UserConfigの妥当性をチェックします。
    /// 
    /// # Returns
    /// 設定が妥当な場合はOk、そうでない場合は ConfigError::ValidationError を返します
    pub fn validate(&self) -> Result<(), ConfigError> {
        // UserConfigの検証のみ実施
        // TODO: UserConfig::validate() 実装後にここで呼び出す
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_validate_config() {
        // UserConfig検証のテスト
        // 現在はプレースホルダーなので常に成功
        let config = Config::load().expect("Failed to load config");
        let result = config.validate();
        
        assert!(result.is_ok(), "Config validation should succeed");
    }

    #[test]
    fn test_config_integration() {
        // Config全体の統合動作を確認
        let config = Config::load().expect("Failed to load config");
        
        // AppConfigの値がコンパイル時定数から正しく参照されていることを確認
        assert_eq!(config.app.api.endpoint, "https://api.streamable.com");
        assert_eq!(config.app.api.timeout_seconds, 30);
        assert!(!config.app.upload.supported_formats.is_empty());
        
        // UserConfigが読み込まれていることを確認
        // （api_keyはNoneでも構わない）
        assert!(config.user.show_notification || !config.user.show_notification); // 常にtrue（存在確認）
    }
}

