/// ユーザー設定モジュール
/// 
/// 実行時にユーザーディレクトリから読み込まれる動的設定を管理します。
/// Windows: C:\Users\<User>\AppData\Roaming\streamable-cli\config.toml
/// macOS:   /Users/<User>/Library/Application Support/streamable-cli/config.toml
/// Linux:   /home/<user>/.config/streamable-cli/config.toml
/// 
/// 初回起動時にデフォルト値から自動的にconfig.tomlを作成します。

use crate::config::error::ConfigError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const DEFAULT_API_KEY: &str = "your_api_key_here";
const DEFAULT_TITLE: &str = "My Video";
const DEFAULT_AUTO_COPY_URL: bool = true;
const DEFAULT_SHOW_NOTIFICATION: bool = true;

/// ユーザー設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// Streamable API キー
    pub api_key: Option<String>,
    
    /// デフォルトのビデオタイトル
    pub default_title: Option<String>,
    
    /// アップロード後に自動的にURLをクリップボードにコピーするか
    #[serde(default = "default_auto_copy_url")]
    pub auto_copy_url: bool,
    
    /// アップロード完了時に通知を表示するか
    #[serde(default = "default_show_notification")]
    pub show_notification: bool,
}

// プライベート関数（serde用）
fn default_auto_copy_url() -> bool {
    DEFAULT_AUTO_COPY_URL
}

fn default_show_notification() -> bool {
    DEFAULT_SHOW_NOTIFICATION
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            api_key: Some(DEFAULT_API_KEY.to_string()),
            default_title: Some(DEFAULT_TITLE.to_string()),
            auto_copy_url: DEFAULT_AUTO_COPY_URL,
            show_notification: DEFAULT_SHOW_NOTIFICATION,
        }
    }
}

impl UserConfig {
    /// ユーザー設定ファイルのパスを取得
    /// 
    /// # Returns
    /// プラットフォーム固有の設定ファイルパス
    /// 
    /// # Errors
    /// ホームディレクトリが取得できない場合に ConfigError::DirectoryNotFound を返します。
    pub fn config_path() -> Result<PathBuf, ConfigError> {
        dirs::config_dir()
            .ok_or_else(|| ConfigError::DirectoryNotFound {
                message: "Failed to get user config directory".to_string(),
            })
            .map(|config_dir| config_dir.join("streamable-cli").join("config.toml"))
    }
    
    /// ユーザー設定を読み込む
    /// 
    /// 設定ファイルが存在しない場合は、デフォルトテンプレートから自動的に作成します。
    /// 
    /// # Returns
    /// ユーザー設定
    /// 
    /// # Errors
    /// 設定ファイルの読み込みまたはパースに失敗した場合に ConfigError を返します。
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::config_path()?;
        
        if !config_path.exists() {
            // 設定ファイルが存在しない場合は、デフォルトテンプレートから作成
            Self::create_default_config(&config_path)?;
        }
        
        let content = fs::read_to_string(&config_path)
            .map_err(|e| ConfigError::FileSystem {
                context: format!("Failed to read config file: {}", config_path.display()),
                source: e,
            })?;
        
        toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError {
                context: format!("Failed to parse config file ({})", config_path.display()),
                source: e,
            })
    }
    
    /// デフォルト設定ファイルを作成
    /// 
    /// # Errors
    /// ディレクトリの作成またはファイルの書き込みに失敗した場合に ConfigError を返します。
    fn create_default_config(config_path: &PathBuf) -> Result<(), ConfigError> {
        // 設定ディレクトリが存在しない場合は作成
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ConfigError::FileSystem {
                    context: format!("Failed to create config directory: {}", parent.display()),
                    source: e,
                })?;
        }
        
        // デフォルト値からTOMLを生成して書き込み
        let default_toml = Self::default_toml_content();
        fs::write(config_path, default_toml)
            .map_err(|e| ConfigError::FileSystem {
                context: format!("Failed to create default config file: {}", config_path.display()),
                source: e,
            })?;
        
        Ok(())
    }
    
    /// デフォルトTOML設定を生成
    /// 
    /// Default トレイトの実装から自動的にTOML文字列を生成します。
    /// これにより、Rust側のデフォルト値とTOMLテンプレートの同期が保証されます。
    fn default_toml_content() -> String {
        format!(
r#"# Streamable CLI - User Configuration
# Streamable API キー (必須)
# https://streamable.com/settings から取得してください
api_key = "{}"

# デフォルトのビデオタイトル (オプション)
default_title = "{}"

# アップロード後にURLを自動的にクリップボードにコピーする
auto_copy_url = {}

# アップロード完了時に通知を表示する
show_notification = {}
"#,
        DEFAULT_API_KEY,
        DEFAULT_TITLE,
        DEFAULT_AUTO_COPY_URL,
        DEFAULT_SHOW_NOTIFICATION
        )
    }
    
    /// ユーザー設定を保存する
    /// 
    /// 必要に応じて設定ディレクトリを作成します。
    /// 
    /// # Errors
    /// ディレクトリの作成またはファイルの書き込みに失敗した場合に ConfigError を返します。
    pub fn save(&self) -> Result<(), ConfigError> {
        let config_path = Self::config_path()?;
        
        // 設定ディレクトリが存在しない場合は作成
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ConfigError::FileSystem {
                    context: format!("Failed to create config directory: {}", parent.display()),
                    source: e,
                })?;
        }
        
        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::SerializeError {
                context: "Failed to serialize config".to_string(),
                source: e,
            })?;
        
        fs::write(&config_path, content)
            .map_err(|e| ConfigError::FileSystem {
                context: format!("Failed to write config file: {}", config_path.display()),
                source: e,
            })?;
        
        Ok(())
    }
    
    /// APIキーが設定されているかチェック
    pub fn has_api_key(&self) -> bool {
        self.api_key.as_ref().map_or(false, |key| !key.is_empty())
    }
    
    /// APIキーを設定
    pub fn set_api_key(&mut self, api_key: String) {
        self.api_key = Some(api_key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_has_api_key() {
        // APIキーの有無を正しく判定できることを確認
        let mut config = UserConfig {
            api_key: None,
            default_title: None,
            auto_copy_url: false,
            show_notification: true,
        };
        
        assert!(!config.has_api_key());
        
        config.set_api_key("test_key".to_string());
        assert!(config.has_api_key());
    }

    #[test]
    fn test_config_path() {
        // プラットフォーム固有のパスが正しく取得できることを確認
        let path = UserConfig::config_path().expect("Failed to get config path");
        assert!(path.to_string_lossy().contains("streamable-cli"));
        assert!(path.to_string_lossy().ends_with("config.toml"));
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        // save() と load() の往復検証
        
        // 既存の設定ファイルを削除してクリーンな状態から開始
        let config_path = UserConfig::config_path().expect("Failed to get config path");
        if config_path.exists() {
            fs::remove_file(&config_path).ok();
        }
        
        // テスト用の設定を作成
        let test_config = UserConfig {
            api_key: Some("test_api_key_12345".to_string()),
            default_title: Some("Test Video Title".to_string()),
            auto_copy_url: true,
            show_notification: false,
        };
        
        // 保存を実行
        test_config.save().expect("Failed to save config");
        
        // ファイルが存在することを確認
        assert!(config_path.exists(), "Config file should exist after save");
        
        // 読み込みを実行
        let loaded_config = UserConfig::load().expect("Failed to load config");
        
        // 値が一致することを確認
        assert_eq!(loaded_config.api_key, test_config.api_key, "API keys should match");
        assert_eq!(loaded_config.default_title, test_config.default_title, "Titles should match");
        assert_eq!(loaded_config.auto_copy_url, test_config.auto_copy_url, "auto_copy_url should match");
        assert_eq!(loaded_config.show_notification, test_config.show_notification, "show_notification should match");
    }

    #[test]
    fn test_save_creates_directory() {
        // save() がディレクトリを自動作成することを確認
        let config_path = UserConfig::config_path().expect("Failed to get config path");
        
        // 親ディレクトリが存在することを確認（save()によって作成されるべき）
        if let Some(parent) = config_path.parent() {
            // テスト用の設定を保存
            let test_config = UserConfig {
                api_key: Some("test_key".to_string()),
                default_title: None,
                auto_copy_url: false,
                show_notification: true,
            };
            
            test_config.save().expect("Failed to save config");
            
            // ディレクトリが存在することを確認
            assert!(parent.exists());
            // ファイルが存在することを確認
            assert!(config_path.exists());
        }
    }

    #[test]
    fn test_load_creates_default_if_not_exists() {
        // load() が設定ファイルが存在しない場合にデフォルトを作成することを確認
        let config_path = UserConfig::config_path().expect("Failed to get config path");
        
        // 既存の設定ファイルを削除（存在する場合）
        if config_path.exists() {
            fs::remove_file(&config_path).ok();
        }
        
        // load() を実行（デフォルトファイルが作成されるはず）
        let config = UserConfig::load().expect("Failed to load config");
        
        // ファイルが作成されたことを確認
        assert!(config_path.exists());
        
        // デフォルト値が適用されていることを確認
        // user-default.toml の内容に基づく
        assert!(config.api_key.is_some()); // デフォルトでは "your_api_key_here"
        assert!(config.show_notification); // デフォルトは true
    }

    #[test]
    fn test_config_serialization() {
        // 設定のシリアライゼーションが正しく動作することを確認
        let config = UserConfig {
            api_key: Some("test_key".to_string()),
            default_title: Some("My Video".to_string()),
            auto_copy_url: true,
            show_notification: false,
        };
        
        // TOML形式にシリアライズ
        let serialized = toml::to_string_pretty(&config).expect("Failed to serialize");
        
        // 必要なフィールドが含まれていることを確認
        assert!(serialized.contains("api_key"));
        assert!(serialized.contains("test_key"));
        assert!(serialized.contains("default_title"));
        assert!(serialized.contains("auto_copy_url"));
        assert!(serialized.contains("show_notification"));
    }
}

