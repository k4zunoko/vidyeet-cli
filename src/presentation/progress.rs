/// プレゼンテーション層: アップロード進捗表示DTO
///
/// ドメイン層の`UploadProgress`をUI表示に適した形式に変換します。
/// この変換により、プレゼンテーション層がドメイン層の実装詳細に
/// 依存しないようにします。

use crate::domain::progress::{UploadProgress, UploadPhase};

/// 進捗表示のカテゴリ
///
/// UIでの表示方法を決定するためのメタ情報
#[derive(Debug, Clone, PartialEq)]
pub enum ProgressCategory {
    /// ファイル検証中
    Validation,
    /// アップロード準備中
    Preparation,
    /// ファイルアップロード中
    Upload,
    /// アセット処理待機中
    Processing,
    /// 完了
    Completed,
}

/// プレゼンテーション層用の進捗情報
///
/// ドメイン層の`UploadProgress`から生成され、
/// UI表示に必要な情報のみを保持します。
#[derive(Debug, Clone)]
pub struct DisplayProgress {
    /// 表示用メッセージ
    pub message: String,
    /// 進捗カテゴリ
    pub category: ProgressCategory,
    /// 詳細情報（オプション）
    pub details: Option<String>,
}

impl DisplayProgress {
    /// 新しい表示用進捗情報を作成
    pub fn new(message: String, category: ProgressCategory) -> Self {
        Self {
            message,
            category,
            details: None,
        }
    }

    /// 詳細情報を追加
    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }
}

/// ドメイン層の`UploadProgress`からプレゼンテーション層の`DisplayProgress`への変換
///
/// Rustの`From`トレイトを実装することで、型安全な変換を実現します。
/// この変換はプレゼンテーション層の責務であり、ドメイン層は
/// `DisplayProgress`の存在を知りません（依存方向の遵守）。
impl From<UploadProgress> for DisplayProgress {
    fn from(progress: UploadProgress) -> Self {
        match progress.phase {
            UploadPhase::ValidatingFile { file_path } => {
                DisplayProgress::new(
                    format!("Validating file: {}", file_path),
                    ProgressCategory::Validation,
                )
            }
            UploadPhase::FileValidated {
                file_name,
                size_bytes,
                format,
            } => {
                let size_mb = size_bytes as f64 / 1_048_576.0;
                DisplayProgress::new(
                    format!("File validated: {} ({:.2} MB, {})", file_name, size_mb, format),
                    ProgressCategory::Validation,
                )
            }
            UploadPhase::CreatingDirectUpload { file_name } => {
                DisplayProgress::new(
                    format!("Creating upload session for: {}", file_name),
                    ProgressCategory::Preparation,
                )
            }
            UploadPhase::DirectUploadCreated { upload_id } => {
                DisplayProgress::new(
                    format!("Upload session created (ID: {})", upload_id),
                    ProgressCategory::Preparation,
                )
            }
            UploadPhase::UploadingFile {
                file_name,
                size_bytes,
            } => {
                let size_mb = size_bytes as f64 / 1_048_576.0;
                DisplayProgress::new(
                    format!("Uploading file: {} ({:.2} MB)...", file_name, size_mb),
                    ProgressCategory::Upload,
                )
            }
            UploadPhase::FileUploaded {
                file_name,
                size_bytes,
            } => {
                let size_mb = size_bytes as f64 / 1_048_576.0;
                DisplayProgress::new(
                    format!("File uploaded: {} ({:.2} MB)", file_name, size_mb),
                    ProgressCategory::Upload,
                )
            }
            UploadPhase::WaitingForAsset {
                upload_id: _,
                elapsed_secs,
            } => {
                if elapsed_secs == 0 {
                    DisplayProgress::new(
                        "Waiting for asset creation...".to_string(),
                        ProgressCategory::Processing,
                    )
                } else if elapsed_secs % 10 == 0 {
                    // 10秒ごとに経過時間を更新
                    DisplayProgress::new(
                        format!("Still waiting... ({}s elapsed)", elapsed_secs),
                        ProgressCategory::Processing,
                    )
                } else {
                    // 10秒未満の更新は無視（表示を抑制）
                    DisplayProgress::new(String::new(), ProgressCategory::Processing)
                }
            }
            UploadPhase::Completed { asset_id } => {
                DisplayProgress::new(
                    format!("Asset created: {}", asset_id),
                    ProgressCategory::Completed,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_progress_creation() {
        let progress = DisplayProgress::new(
            "Test message".to_string(),
            ProgressCategory::Validation,
        );

        assert_eq!(progress.message, "Test message");
        assert_eq!(progress.category, ProgressCategory::Validation);
        assert!(progress.details.is_none());
    }

    #[test]
    fn test_display_progress_with_details() {
        let progress = DisplayProgress::new(
            "Test message".to_string(),
            ProgressCategory::Upload,
        )
        .with_details("Additional info".to_string());

        assert_eq!(progress.details, Some("Additional info".to_string()));
    }

    #[test]
    fn test_from_upload_progress_validating_file() {
        let domain_progress = UploadProgress::new(UploadPhase::ValidatingFile {
            file_path: "/path/to/file.mp4".to_string(),
        });

        let display_progress: DisplayProgress = domain_progress.into();

        assert_eq!(
            display_progress.message,
            "Validating file: /path/to/file.mp4"
        );
        assert_eq!(display_progress.category, ProgressCategory::Validation);
    }

    #[test]
    fn test_from_upload_progress_file_validated() {
        let domain_progress = UploadProgress::new(UploadPhase::FileValidated {
            file_name: "video.mp4".to_string(),
            size_bytes: 10_485_760, // 10 MB
            format: "mp4".to_string(),
        });

        let display_progress: DisplayProgress = domain_progress.into();

        assert!(display_progress.message.contains("video.mp4"));
        assert!(display_progress.message.contains("10.00 MB"));
        assert_eq!(display_progress.category, ProgressCategory::Validation);
    }

    #[test]
    fn test_from_upload_progress_waiting_initial() {
        let domain_progress = UploadProgress::new(UploadPhase::WaitingForAsset {
            upload_id: "test_id".to_string(),
            elapsed_secs: 0,
        });

        let display_progress: DisplayProgress = domain_progress.into();

        assert_eq!(display_progress.message, "Waiting for asset creation...");
        assert_eq!(display_progress.category, ProgressCategory::Processing);
    }

    #[test]
    fn test_from_upload_progress_waiting_elapsed() {
        let domain_progress = UploadProgress::new(UploadPhase::WaitingForAsset {
            upload_id: "test_id".to_string(),
            elapsed_secs: 20,
        });

        let display_progress: DisplayProgress = domain_progress.into();

        assert_eq!(display_progress.message, "Still waiting... (20s elapsed)");
        assert_eq!(display_progress.category, ProgressCategory::Processing);
    }

    #[test]
    fn test_from_upload_progress_waiting_suppressed() {
        // 10秒未満の更新は表示を抑制
        let domain_progress = UploadProgress::new(UploadPhase::WaitingForAsset {
            upload_id: "test_id".to_string(),
            elapsed_secs: 5,
        });

        let display_progress: DisplayProgress = domain_progress.into();

        assert_eq!(display_progress.message, "");
        assert_eq!(display_progress.category, ProgressCategory::Processing);
    }

    #[test]
    fn test_from_upload_progress_completed() {
        let domain_progress = UploadProgress::new(UploadPhase::Completed {
            asset_id: "asset_123".to_string(),
        });

        let display_progress: DisplayProgress = domain_progress.into();

        assert_eq!(display_progress.message, "Asset created: asset_123");
        assert_eq!(display_progress.category, ProgressCategory::Completed);
    }
}
