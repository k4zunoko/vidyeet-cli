/// アプリケーション設定モジュール
///
/// ビルド時にコンパイル時定数として定義される静的設定を管理します。
/// これらの設定は実行時には変更できません。

/// アプリケーション全体の設定
#[derive(Debug, Clone, Copy)]
pub struct AppConfig {
    pub api: ApiConfig,
    pub upload: UploadConfig,
    pub logging: LoggingConfig,
}

/// API関連の設定
#[derive(Debug, Clone, Copy)]
pub struct ApiConfig {
    /// Mux Video API のベースURL
    pub endpoint: &'static str,

    /// APIリクエストのタイムアウト(秒)
    pub timeout_seconds: u64,

    /// 最大リトライ回数
    pub max_retries: u32,
}

/// アップロード関連の設定
#[derive(Debug, Clone, Copy)]
pub struct UploadConfig {
    /// アップロード可能な最大ファイルサイズ (バイト)
    pub max_file_size: u64,

    /// アップロードのチャンクサイズ (バイト)
    pub chunk_size: u64,

    /// 対応する動画フォーマット
    pub supported_formats: &'static [&'static str],

    /// アップロード完了ポーリング間隔(秒)
    pub poll_interval_secs: u64,

    /// アップロード待機の最大時間(秒)
    pub max_wait_secs: u64,
}

/// ロギング関連の設定
#[derive(Debug, Clone, Copy)]
pub struct LoggingConfig {
    /// ログレベル (trace, debug, info, warn, error)
    pub level: &'static str,

    /// ログファイルの保存先 (空の場合は標準出力のみ)
    pub file_path: &'static str,
}

impl AppConfig {
    /// コンパイル時定数として設定を構築
    pub const fn new() -> Self {
        Self {
            api: ApiConfig {
                endpoint: "https://api.mux.com",
                timeout_seconds: 300, // 5分（大きなファイルアップロード用）
                max_retries: 3,
            },
            upload: UploadConfig {
                max_file_size: 10_737_418_240, // 10GB
                chunk_size: 10_485_760,        // 10MB
                supported_formats: &["mp4", "mov", "avi", "wmv", "flv", "mkv", "webm"],
                poll_interval_secs: 2,
                max_wait_secs: 300,
            },
            logging: LoggingConfig {
                level: "info",
                file_path: "",
            },
        }
    }
}

/// アプリケーション設定のグローバル定数
///
/// コンパイル時に評価され、実行時のコストはゼロです。
pub const APP_CONFIG: AppConfig = AppConfig::new();

impl UploadConfig {
    /// 拡張子からContent-Typeを取得
    ///
    /// # 引数
    /// * `extension` - ファイル拡張子（例: "mp4", "mov"）
    ///
    /// # 戻り値
    /// Content-Type文字列、サポートされていない場合は "application/octet-stream"
    pub fn get_content_type(&self, extension: &str) -> &'static str {
        match extension {
            "mp4" => "video/mp4",
            "mov" => "video/quicktime",
            "avi" => "video/x-msvideo",
            "wmv" => "video/x-ms-wmv",
            "flv" => "video/x-flv",
            "mkv" => "video/x-matroska",
            "webm" => "video/webm",
            _ => "application/octet-stream",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_constants() {
        // グローバル定数が正しく定義されていることを確認
        assert_eq!(APP_CONFIG.api.endpoint, "https://api.mux.com");
        assert_eq!(APP_CONFIG.api.timeout_seconds, 300);
        assert_eq!(APP_CONFIG.api.max_retries, 3);
        assert!(!APP_CONFIG.upload.supported_formats.is_empty());
    }

    #[test]
    fn test_app_config_values() {
        // 各設定値が期待通りであることを確認
        assert_eq!(APP_CONFIG.upload.max_file_size, 10_737_418_240); // 10GB
        assert_eq!(APP_CONFIG.upload.chunk_size, 10_485_760);
        assert_eq!(APP_CONFIG.upload.supported_formats.len(), 7);
        assert_eq!(APP_CONFIG.logging.level, "info");
        assert_eq!(APP_CONFIG.logging.file_path, "");
    }

    #[test]
    fn test_supported_formats() {
        // サポートされているフォーマットの確認
        let formats = APP_CONFIG.upload.supported_formats;
        assert!(formats.contains(&"mp4"));
        assert!(formats.contains(&"mov"));
        assert!(formats.contains(&"webm"));
    }

    #[test]
    fn test_get_content_type() {
        // Content-Type変換が正しく動作することを確認
        let upload_config = &APP_CONFIG.upload;
        
        assert_eq!(upload_config.get_content_type("mp4"), "video/mp4");
        assert_eq!(upload_config.get_content_type("mov"), "video/quicktime");
        assert_eq!(upload_config.get_content_type("avi"), "video/x-msvideo");
        assert_eq!(upload_config.get_content_type("wmv"), "video/x-ms-wmv");
        assert_eq!(upload_config.get_content_type("flv"), "video/x-flv");
        assert_eq!(upload_config.get_content_type("mkv"), "video/x-matroska");
        assert_eq!(upload_config.get_content_type("webm"), "video/webm");
        
        // サポートされていない拡張子
        assert_eq!(upload_config.get_content_type("unknown"), "application/octet-stream");
        assert_eq!(upload_config.get_content_type("txt"), "application/octet-stream");
    }
}
