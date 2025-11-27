/// ユーザー設定モジュール
///
/// 実行時にユーザーディレクトリから読み込まれる動的設定を管理します。
/// Windows: C:\Users\<User>\AppData\Roaming\vidyeet\config.toml
/// macOS:   /Users/<User>/Library/Application Support/vidyeet/config.toml
/// Linux:   /home/<user>/.config/vidyeet/config.toml
///
/// 初回起動時にデフォルト値から自動的にconfig.tomlを作成します。
use crate::config::error::ConfigError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const DEFAULT_TITLE: &str = "My Video";
const DEFAULT_AUTO_COPY_URL: bool = false;
const DEFAULT_SHOW_NOTIFICATION: bool = false;

/// Mux認証設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Mux Access Token ID
    pub token_id: String,

    /// Mux Access Token Secret
    pub token_secret: String,
}

/// ユーザー設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// デフォルトのビデオタイトル
    pub default_title: Option<String>,

    /// Mux認証情報
    pub auth: Option<AuthConfig>,

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
            default_title: Some(DEFAULT_TITLE.to_string()),
            auth: None,
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
            .map(|config_dir| config_dir.join("vidyeet").join("config.toml"))
    }

    /// ユーザー設定を読み込む
    ///
    /// 設定ファイルが存在しない場合は、デフォルトテンプレートから自動的に作成します。
    /// 読み込み後、自動的に検証を実行します（Fail Fast）。
    ///
    /// # Returns
    /// 検証済みのユーザー設定
    ///
    /// # Errors
    /// 設定ファイルの読み込み、パース、または検証に失敗した場合に ConfigError を返します。
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            // 設定ファイルが存在しない場合は、デフォルトテンプレートから作成
            Self::create_default_config(&config_path)?;
        }

        let content = fs::read_to_string(&config_path).map_err(|e| ConfigError::FileSystem {
            context: format!("Failed to read config file: {}", config_path.display()),
            source: e,
        })?;

        let config: Self = toml::from_str(&content).map_err(|e| ConfigError::ParseError {
            context: format!("Failed to parse config file ({})", config_path.display()),
            source: e,
        })?;

        // 自動検証（Fail Fast）
        config.validate()?;

        Ok(config)
    }

    /// デフォルト設定ファイルを作成
    ///
    /// # Errors
    /// ディレクトリの作成またはファイルの書き込みに失敗した場合に ConfigError を返します。
    fn create_default_config(config_path: &PathBuf) -> Result<(), ConfigError> {
        // 設定ディレクトリが存在しない場合は作成
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::FileSystem {
                context: format!("Failed to create config directory: {}", parent.display()),
                source: e,
            })?;
        }

        // デフォルト値からTOMLを生成して書き込み
        let default_toml = Self::default_toml_content();
        fs::write(config_path, default_toml).map_err(|e| ConfigError::FileSystem {
            context: format!(
                "Failed to create default config file: {}",
                config_path.display()
            ),
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
            r#"# Mux Video CLI - User Configuration
# 認証情報は 'vidyeet login' で設定されます
default_title = "{}"
auto_copy_url = {}
show_notification = {}
"#,
            DEFAULT_TITLE, DEFAULT_AUTO_COPY_URL, DEFAULT_SHOW_NOTIFICATION
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
            fs::create_dir_all(parent).map_err(|e| ConfigError::FileSystem {
                context: format!("Failed to create config directory: {}", parent.display()),
                source: e,
            })?;
        }

        let content = toml::to_string_pretty(self).map_err(|e| ConfigError::SerializeError {
            context: "Failed to serialize config".to_string(),
            source: e,
        })?;

        fs::write(&config_path, content).map_err(|e| ConfigError::FileSystem {
            context: format!("Failed to write config file: {}", config_path.display()),
            source: e,
        })?;

        // Unix系: パーミッション設定 (0600) - トークンが含まれるため
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o600);
            fs::set_permissions(&config_path, permissions).map_err(|e| {
                ConfigError::FileSystem {
                    context: format!(
                        "Failed to set permissions for config file: {}",
                        config_path.display()
                    ),
                    source: e,
                }
            })?;
        }

        Ok(())
    }

    /// ユーザー設定を検証
    ///
    /// Fail Fast: 設定に問題がある場合は即座にエラーを返します。
    ///
    /// # 検証内容
    /// - 現在は特に検証項目はありません
    ///   (認証は login コマンドで管理されます)
    ///
    /// # Errors
    /// 検証に失敗した場合に ConfigError::ValidationError を返します。
    pub fn validate(&self) -> Result<(), ConfigError> {
        // 認証は login コマンドで管理されるため、ここでの検証は不要
        Ok(())
    }

    /// 認証情報を設定
    pub fn set_auth(&mut self, token_id: String, token_secret: String) {
        self.auth = Some(AuthConfig {
            token_id,
            token_secret,
        });
    }

    /// 認証情報を取得
    ///
    /// # Errors
    /// 認証情報が設定されていない場合に ConfigError::TokenNotFound を返します。
    pub fn get_auth(&self) -> Result<&AuthConfig, ConfigError> {
        self.auth
            .as_ref()
            .ok_or_else(|| ConfigError::TokenNotFound {
                message: "Authentication credentials not found. Please run 'vidyeet login' first.".to_string(),
            })
    }

    /// 認証情報が存在するかチェック
    pub fn has_auth(&self) -> bool {
        self.auth.is_some()
    }

    /// 認証情報を削除
    pub fn clear_auth(&mut self) {
        self.auth = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_has_auth() {
        // 認証情報の有無を正しく判定できることを確認
        let mut config = UserConfig {
            default_title: None,
            auth: None,
            auto_copy_url: false,
            show_notification: true,
        };

        assert!(!config.has_auth());

        config.set_auth("test_token_id".to_string(), "test_token_secret".to_string());
        assert!(config.has_auth());
    }

    #[test]
    fn test_get_auth() {
        // 認証情報の取得が正しく動作することを確認
        let mut config = UserConfig::default();
        
        // 認証情報が未設定の場合はエラー
        let result = config.get_auth();
        assert!(result.is_err());
        if let Err(ConfigError::TokenNotFound { message }) = result {
            assert!(message.contains("login"));
        }
        
        // 認証情報設定後は取得できる
        config.set_auth("test_id".to_string(), "test_secret".to_string());
        let auth = config.get_auth().unwrap();
        assert_eq!(auth.token_id, "test_id");
        assert_eq!(auth.token_secret, "test_secret");
    }

    #[test]
    fn test_clear_auth() {
        // 認証情報のクリアが正しく動作することを確認
        let mut config = UserConfig::default();
        config.set_auth("test_id".to_string(), "test_secret".to_string());
        
        assert!(config.has_auth());
        
        config.clear_auth();
        assert!(!config.has_auth());
        assert!(config.get_auth().is_err());
    }

    #[test]
    fn test_config_path() {
        // プラットフォーム固有のパスが正しく取得できることを確認
        let path = UserConfig::config_path().expect("Failed to get config path");
        assert!(path.to_string_lossy().contains("vidyeet"));
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
        let mut test_config = UserConfig {
            default_title: Some("Test Video Title".to_string()),
            auth: None,
            auto_copy_url: true,
            show_notification: false,
        };
        test_config.set_auth("test_id_xyz".to_string(), "test_secret_xyz".to_string());

        // 保存を実行
        test_config.save().expect("Failed to save config");

        // ファイルが存在することを確認
        assert!(config_path.exists(), "Config file should exist after save");

        // 読み込みを実行
        let loaded_config = UserConfig::load().expect("Failed to load config");

        // 値が一致することを確認
        let loaded_auth = loaded_config.get_auth().expect("Auth should be present");
        let test_auth = test_config.get_auth().expect("Auth should be present");
        assert_eq!(
            loaded_auth.token_id, test_auth.token_id,
            "Token IDs should match"
        );
        assert_eq!(
            loaded_auth.token_secret, test_auth.token_secret,
            "Token secrets should match"
        );
        assert_eq!(
            loaded_config.default_title, test_config.default_title,
            "Titles should match"
        );
        assert_eq!(
            loaded_config.auto_copy_url, test_config.auto_copy_url,
            "auto_copy_url should match"
        );
        assert_eq!(
            loaded_config.show_notification, test_config.show_notification,
            "show_notification should match"
        );
    }

    #[test]
    fn test_save_creates_directory() {
        // save() がディレクトリを自動作成することを確認
        let config_path = UserConfig::config_path().expect("Failed to get config path");

        // 親ディレクトリが存在することを確認（save()によって作成されるべき）
        if let Some(parent) = config_path.parent() {
            // テスト用の設定を保存
            let test_config = UserConfig {
                default_title: None,
                auth: Some(AuthConfig {
                    token_id: "test_token_id".to_string(),
                    token_secret: "test_token_secret".to_string(),
                }),
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

        // load() を実行（デフォルトファイルが作成される）
        let result = UserConfig::load();

        // ファイルが作成されたことを確認
        assert!(config_path.exists(), "Config file should be created");

        // 認証なしでもロード成功（検証不要）
        assert!(result.is_ok(), "Default config should load successfully");

        // ファイルの内容を直接読んでデフォルト値が書かれていることを確認
        let content = fs::read_to_string(&config_path).expect("Failed to read config");
        assert!(content.contains("auto_copy_url"));
        assert!(content.contains("vidyeet login"));
    }

    #[test]
    fn test_config_serialization() {
        // 設定のシリアライゼーションが正しく動作することを確認
        let config = UserConfig {
            default_title: Some("My Video".to_string()),
            auth: Some(AuthConfig {
                token_id: "test_token_id".to_string(),
                token_secret: "test_token_secret".to_string(),
            }),
            auto_copy_url: true,
            show_notification: false,
        };

        // TOML形式にシリアライズ
        let serialized = toml::to_string_pretty(&config).expect("Failed to serialize");

        // 必要なフィールドが含まれていることを確認
        assert!(serialized.contains("auth"));
        assert!(serialized.contains("token_id"));
        assert!(serialized.contains("token_secret"));
        assert!(serialized.contains("default_title"));
        assert!(serialized.contains("auto_copy_url"));
        assert!(serialized.contains("show_notification"));
    }

    #[test]
    fn test_validate_always_succeeds() {
        // 認証はloginコマンドで管理されるため、validate()は常に成功する
        let config = UserConfig {
            default_title: None,
            auth: None,
            auto_copy_url: false,
            show_notification: true,
        };

        let result = config.validate();
        assert!(result.is_ok());
    }
}
