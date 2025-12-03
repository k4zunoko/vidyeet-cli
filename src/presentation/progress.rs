/// プレゼンテーション層: アップロード進捗表示DTO
///
/// ドメイン層の`UploadProgress`をUI表示に適した形式に変換します。
/// この変換により、プレゼンテーション層がドメイン層の実装詳細に
/// 依存しないようにします。
///
/// # 設計方針
/// - `From<&UploadProgress>`で借用による変換（所有権を奪わない）
/// - `Option<DisplayProgress>`で表示抑制を明示的に表現
/// - ヘルパー関数で各フェーズの変換ロジックを分離（密結合緩和）

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
/// # 設計改善
/// - `&UploadProgress`を借用して変換（所有権を奪わない）
/// - `Option<DisplayProgress>`を返すことで、表示しないケースを明示的に表現
/// - ヘルパー関数で各フェーズの変換ロジックを分離し、密結合を緩和
///
/// # 戻り値
/// - `Some(DisplayProgress)`: 表示すべき進捗情報
/// - `None`: 表示を抑制（例: 10秒未満の経過時間更新）
impl From<&UploadProgress> for Option<DisplayProgress> {
    fn from(progress: &UploadProgress) -> Self {
        match &progress.phase {
            UploadPhase::ValidatingFile { file_path } => {
                Some(format_validating_file(file_path))
            }
            UploadPhase::FileValidated { file_name, size_bytes, format } => {
                Some(format_file_validated(file_name, *size_bytes, format))
            }
            UploadPhase::CreatingDirectUpload { file_name } => {
                Some(format_creating_upload(file_name))
            }
            UploadPhase::DirectUploadCreated { upload_id } => {
                Some(format_upload_created(upload_id))
            }
            UploadPhase::UploadingFile { file_name, size_bytes } => {
                Some(format_uploading_file(file_name, *size_bytes))
            }
            UploadPhase::FileUploaded { file_name, size_bytes } => {
                Some(format_file_uploaded(file_name, *size_bytes))
            }
            UploadPhase::WaitingForAsset { elapsed_secs, .. } => {
                format_waiting_for_asset(*elapsed_secs)
            }
            UploadPhase::Completed { asset_id } => {
                Some(format_completed(asset_id))
            }
        }
    }
}

// ============================================================================
// ヘルパー関数: 各フェーズの変換ロジック
// ============================================================================
// 各フェーズの変換ロジックを独立した関数に分離することで：
// - match文の複雑度を削減
// - 各フェーズの変換ロジックをテスト可能に
// - 将来のフェーズ追加時の影響範囲を最小化

fn format_validating_file(file_path: &str) -> DisplayProgress {
    DisplayProgress::new(
        format!("Validating file: {}", file_path),
        ProgressCategory::Validation,
    )
}

fn format_file_validated(file_name: &str, size_bytes: u64, format: &str) -> DisplayProgress {
    let size_mb = size_bytes as f64 / 1_048_576.0;
    DisplayProgress::new(
        format!("File validated: {} ({:.2} MB, {})", file_name, size_mb, format),
        ProgressCategory::Validation,
    )
}

fn format_creating_upload(file_name: &str) -> DisplayProgress {
    DisplayProgress::new(
        format!("Creating upload session for: {}", file_name),
        ProgressCategory::Preparation,
    )
}

fn format_upload_created(upload_id: &str) -> DisplayProgress {
    DisplayProgress::new(
        format!("Upload session created (ID: {})", upload_id),
        ProgressCategory::Preparation,
    )
}

fn format_uploading_file(file_name: &str, size_bytes: u64) -> DisplayProgress {
    let size_mb = size_bytes as f64 / 1_048_576.0;
    DisplayProgress::new(
        format!("Uploading file: {} ({:.2} MB)...", file_name, size_mb),
        ProgressCategory::Upload,
    )
}

fn format_file_uploaded(file_name: &str, size_bytes: u64) -> DisplayProgress {
    let size_mb = size_bytes as f64 / 1_048_576.0;
    DisplayProgress::new(
        format!("File uploaded: {} ({:.2} MB)", file_name, size_mb),
        ProgressCategory::Upload,
    )
}

/// アセット待機中の進捗表示
/// 
/// 10秒ごとにのみ更新を表示し、それ以外は`None`を返すことで
/// 過度な更新を抑制します。
fn format_waiting_for_asset(elapsed_secs: u64) -> Option<DisplayProgress> {
    if elapsed_secs == 0 {
        Some(DisplayProgress::new(
            "Waiting for asset creation...".to_string(),
            ProgressCategory::Processing,
        ))
    } else if elapsed_secs % 10 == 0 {
        Some(DisplayProgress::new(
            format!("Still waiting... ({}s elapsed)", elapsed_secs),
            ProgressCategory::Processing,
        ))
    } else {
        // 10秒未満の更新は表示しない（明示的にNoneを返す）
        None
    }
}

fn format_completed(asset_id: &str) -> DisplayProgress {
    DisplayProgress::new(
        format!("Asset created: {}", asset_id),
        ProgressCategory::Completed,
    )
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

        let display_progress = Option::<DisplayProgress>::from(&domain_progress)
            .expect("update should be displayed");

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

        let display_progress = Option::<DisplayProgress>::from(&domain_progress)
            .expect("update should be displayed");

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

        let display_progress = Option::<DisplayProgress>::from(&domain_progress)
            .expect("update should be displayed");

        assert_eq!(display_progress.message, "Waiting for asset creation...");
        assert_eq!(display_progress.category, ProgressCategory::Processing);
    }

    #[test]
    fn test_from_upload_progress_waiting_elapsed() {
        let domain_progress = UploadProgress::new(UploadPhase::WaitingForAsset {
            upload_id: "test_id".to_string(),
            elapsed_secs: 20,
        });

        let display_progress = Option::<DisplayProgress>::from(&domain_progress)
            .expect("update should be displayed");

        assert_eq!(display_progress.message, "Still waiting... (20s elapsed)");
        assert_eq!(display_progress.category, ProgressCategory::Processing);
    }

    #[test]
    fn test_from_upload_progress_waiting_suppressed() {
        // 10秒未満の更新は表示を抑制（Noneを返す）
        let domain_progress = UploadProgress::new(UploadPhase::WaitingForAsset {
            upload_id: "test_id".to_string(),
            elapsed_secs: 5,
        });

        let display_progress = Option::<DisplayProgress>::from(&domain_progress);

        // None（表示抑制）が返されることを確認
        assert!(display_progress.is_none(), "Updates under 10 seconds should be suppressed");
    }

    #[test]
    fn test_from_upload_progress_completed() {
        let domain_progress = UploadProgress::new(UploadPhase::Completed {
            asset_id: "asset_123".to_string(),
        });

        let display_progress = Option::<DisplayProgress>::from(&domain_progress)
            .expect("update should be displayed");

        assert_eq!(display_progress.message, "Asset created: asset_123");
        assert_eq!(display_progress.category, ProgressCategory::Completed);
    }
}
