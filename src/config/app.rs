/// アプリケーション設定モジュール
/// 
/// ビルド時に config.toml から読み込まれる静的設定を管理します。
/// これらの設定は実行時には変更できません。

use serde::Deserialize;

/// アプリケーション全体の設定
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub api: ApiConfig,
    pub upload: UploadConfig,
    pub logging: LoggingConfig,
}

/// API関連の設定
#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    /// Streamable API のベースURL
    pub endpoint: String,
    
    /// APIリクエストのタイムアウト(秒)
    pub timeout_seconds: u64,
    
    /// 最大リトライ回数
    pub max_retries: u32,
}

/// アップロード関連の設定
#[derive(Debug, Clone, Deserialize)]
pub struct UploadConfig {
    /// アップロード可能な最大ファイルサイズ (バイト)
    pub max_file_size: u64,
    
    /// アップロードのチャンクサイズ (バイト)
    pub chunk_size: u64,
    
    /// 対応する動画フォーマット
    pub supported_formats: Vec<String>,
}

/// ロギング関連の設定
#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    /// ログレベル (trace, debug, info, warn, error)
    pub level: String,
    
    /// ログファイルの保存先 (空の場合は標準出力のみ)
    pub file_path: String,
}

impl AppConfig {
    /// ビルド時に埋め込まれたconfig.tomlから設定を読み込む
    /// 
    /// # Panics
    /// 設定ファイルのパースに失敗した場合はパニックします。
    /// これはビルド時設定なので、実行時エラーではなくコンパイルエラーとして扱うべきです。
    pub fn load() -> Self {
        const CONFIG_STR: &str = include_str!("../../config.toml");
        toml::from_str(CONFIG_STR)
            .expect("Failed to parse embedded config.toml. This is a build-time configuration error.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        // ビルド時設定が正しく読み込まれることを確認
        let config = AppConfig::load();
        assert_eq!(config.api.endpoint, "https://api.streamable.com");
        assert_eq!(config.api.timeout_seconds, 30);
        assert!(!config.upload.supported_formats.is_empty());
    }
}
