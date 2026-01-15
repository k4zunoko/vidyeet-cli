/// プレゼンテーション層: アップロード進捗表示DTO
///
/// ドメイン層の`UploadProgress`をUI表示に適した形式に変換します。
/// この変換により、プレゼンテーション層がドメイン層の実装詳細に
/// 依存しないようにします。
///
/// # 設計方針
/// - 自前トレイト`ToDisplay`で借用による変換（標準Fromトレイトはオーファンルール違反のため使用不可）
/// - `Option<DisplayProgress>`で表示抑制を明示的に表現
/// - ヘルパー関数で各フェーズの変換ロジックを分離（密結合緩和）
/// - 進捗受信ループの処理もこのモジュールで管理（プレゼンテーション層の責務）
use crate::config::{APP_CONFIG, BYTES_PER_MB};
use crate::domain::progress::{UploadPhase, UploadProgress};
use anyhow::Result;

/// ドメイン型からプレゼンテーション表示型への変換トレイト
///
/// # 設計意図
/// 標準ライブラリの`From`トレイトは使用できません（オーファンルール違反）。
/// `impl From<&UploadProgress> for Option<DisplayProgress>`は、
/// 外部トレイト（From）を外部型（Option）に実装することになり、
/// Rustのトレイト孤児規則に違反します。
///
/// そのため、自前トレイト`ToDisplay`を定義して変換を実装します。
pub trait ToDisplay {
    /// 表示用進捗情報に変換
    ///
    /// # 戻り値
    /// - `Some(DisplayProgress)`: 表示すべき進捗情報
    /// - `None`: 表示を抑制（例: 10秒未満の経過時間更新）
    fn to_display(&self) -> Option<DisplayProgress>;
}

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
#[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }
}

/// アップロード進捗を人間向けに表示（stderr）
///
/// プレゼンテーション層の責務として、DisplayProgressを受け取り、
/// ユーザーフレンドリーなメッセージを表示します。
/// ドメイン層の実装詳細（UploadPhase）には依存しません。
pub fn display_upload_progress(progress: &DisplayProgress) {
    // Option<DisplayProgress>により表示すべき進捗のみ渡されるため、
    // 空文字チェック不要（呼び出し側でif let Some()によりフィルタ済み）
    eprintln!("{}", progress.message);
}

/// アップロード進捗を受信して表示するループ処理
///
/// プレゼンテーション層の責務として、進捗チャネルから受信した
/// ドメイン層の進捗情報を表示用に変換し、ユーザーに表示します。
///
/// # 引数
/// * `progress_rx` - 進捗受信チャネル
/// * `machine_output` - 機械可読出力フラグ（true時は機械向けJSON出力）
/// * `show_progress` - 進捗表示フラグ（false時は進捗を完全に抑制）
///
/// # 戻り値
/// 処理が正常に完了した場合は`Ok(())`、タイムアウトした場合は警告を出力
pub async fn handle_upload_progress(
    mut progress_rx: tokio::sync::mpsc::Receiver<UploadProgress>,
    machine_output: bool,
    show_progress: bool,
) -> Result<()> {
    // タイムアウトを設定して無限待機を防ぐ
    use tokio::time::{Duration, timeout};
    let progress_timeout = Duration::from_secs(APP_CONFIG.upload.progress_timeout_secs);

    loop {
        match timeout(progress_timeout, progress_rx.recv()).await {
            Ok(Some(progress)) => {
                if !show_progress {
                    // --progress フラグが指定されていない場合は進捗を表示しない
                    continue;
                }

                if machine_output {
                    // 機械可読JSON出力（stdout）
                    // JSONL形式（1行1JSON）で出力
                    if let Ok(json) = serde_json::to_string(&progress.phase) {
                        println!("{}", json);
                    }
                } else {
                    // 人間向け進捗表示（stderr）
                    // ドメイン層の型をプレゼンテーション層の型に変換（借用）
                    // Option<DisplayProgress>を返すため、表示が必要な場合のみ出力
                    if let Some(display_progress) = progress.to_display() {
                        display_upload_progress(&display_progress);
                    }
                    // Noneの場合は表示を抑制（10秒未満の経過時間更新など）
                }
            }
            Ok(std::option::Option::None) => {
                // チャネルがクローズされた（正常終了）
                break;
            }
            Err(_) => {
                // タイムアウト発生
                eprintln!("Warning: Progress update timed out");
                break;
            }
        }
    }

    Ok(())
}

/// ドメイン層の`UploadProgress`からプレゼンテーション層の`DisplayProgress`への変換
///
/// # 設計改善
/// - `&UploadProgress`を借用して変換（所有権を奪わない）
/// - `Option<DisplayProgress>`を返すことで、表示しないケースを明示的に表現
/// - ヘルパー関数で各フェーズの変換ロジックを分離し、密結合を緩和
///
/// # オーファンルール対応
/// `impl From<&UploadProgress> for Option<DisplayProgress>`は、
/// 外部トレイト（From）を外部型（Option）に実装するため、
/// Rustのトレイト孤児規則に違反します。そのため自前トレイトを使用します。
impl ToDisplay for UploadProgress {
    fn to_display(&self) -> Option<DisplayProgress> {
        match &self.phase {
            UploadPhase::ValidatingFile { file_path } => Some(format_validating_file(file_path)),
            UploadPhase::FileValidated {
                file_name,
                size_bytes,
                format,
            } => Some(format_file_validated(file_name, *size_bytes, format)),
            UploadPhase::CreatingDirectUpload { file_name } => {
                Some(format_creating_upload(file_name))
            }
            UploadPhase::DirectUploadCreated { upload_id } => {
                Some(format_upload_created(upload_id))
            }
            UploadPhase::UploadingFile {
                file_name,
                size_bytes,
            } => Some(format_uploading_file(file_name, *size_bytes)),
            UploadPhase::UploadingChunk {
                current_chunk,
                total_chunks,
                bytes_sent,
                total_bytes,
            } => Some(format_uploading_chunk(
                *current_chunk,
                *total_chunks,
                *bytes_sent,
                *total_bytes,
            )),
            UploadPhase::FileUploaded {
                file_name,
                size_bytes,
            } => Some(format_file_uploaded(file_name, *size_bytes)),
            UploadPhase::WaitingForAsset { elapsed_secs, .. } => {
                format_waiting_for_asset(*elapsed_secs)
            }
            UploadPhase::Completed { asset_id } => Some(format_completed(asset_id)),
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

/// ファイル検証完了時の進捗表示を生成
fn format_file_validated(file_name: &str, size_bytes: u64, format: &str) -> DisplayProgress {
    let size_mb = size_bytes as f64 / BYTES_PER_MB;
    let precision = APP_CONFIG.presentation.size_display_precision;
    DisplayProgress::new(
        format!(
            "File validated: {} ({:.prec$} MB, {})",
            file_name,
            size_mb,
            format,
            prec = precision
        ),
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

/// アップロード開始時の進捗表示を生成
fn format_uploading_file(file_name: &str, size_bytes: u64) -> DisplayProgress {
    let size_mb = size_bytes as f64 / BYTES_PER_MB;
    let precision = APP_CONFIG.presentation.size_display_precision;
    DisplayProgress::new(
        format!(
            "Uploading file: {} ({:.prec$} MB)...",
            file_name,
            size_mb,
            prec = precision
        ),
        ProgressCategory::Upload,
    )
}

/// チャンクアップロード中の進捗表示を生成
///
/// 例: "Uploading chunk 2/5 (64.00 MB / 160.00 MB, 40%)"
fn format_uploading_chunk(
    current_chunk: usize,
    total_chunks: usize,
    bytes_sent: u64,
    total_bytes: u64,
) -> DisplayProgress {
    let sent_mb = bytes_sent as f64 / BYTES_PER_MB;
    let total_mb = total_bytes as f64 / BYTES_PER_MB;
    let percentage = (bytes_sent as f64 / total_bytes as f64 * 100.0) as u8;
    let precision = APP_CONFIG.presentation.size_display_precision;

    DisplayProgress::new(
        format!(
            "Uploading chunk {}/{} ({:.prec$} MB / {:.prec$} MB, {}%)",
            current_chunk,
            total_chunks,
            sent_mb,
            total_mb,
            percentage,
            prec = precision
        ),
        ProgressCategory::Upload,
    )
}

/// アップロード完了時の進捗表示を生成
fn format_file_uploaded(file_name: &str, size_bytes: u64) -> DisplayProgress {
    let size_mb = size_bytes as f64 / BYTES_PER_MB;
    let precision = APP_CONFIG.presentation.size_display_precision;
    DisplayProgress::new(
        format!(
            "File uploaded: {} ({:.prec$} MB)",
            file_name,
            size_mb,
            prec = precision
        ),
        ProgressCategory::Upload,
    )
}

/// アセット待機中の進捗表示
///
/// 設定された間隔（progress_update_interval_secs）ごとにのみ更新を表示し、
/// それ以外は`None`を返すことで過度な更新を抑制します。
fn format_waiting_for_asset(elapsed_secs: u64) -> Option<DisplayProgress> {
    let update_interval = APP_CONFIG.presentation.progress_update_interval_secs;

    if elapsed_secs == 0 {
        Some(DisplayProgress::new(
            "Waiting for asset creation...".to_string(),
            ProgressCategory::Processing,
        ))
    } else if elapsed_secs.is_multiple_of(update_interval) {
        Some(DisplayProgress::new(
            format!("Still waiting... ({}s elapsed)", elapsed_secs),
            ProgressCategory::Processing,
        ))
    } else {
        // 設定間隔未満の更新は表示しない（明示的にNoneを返す）
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
        let progress =
            DisplayProgress::new("Test message".to_string(), ProgressCategory::Validation);

        assert_eq!(progress.message, "Test message");
        assert_eq!(progress.category, ProgressCategory::Validation);
        assert!(progress.details.is_none());
    }

    #[test]
    fn test_display_progress_with_details() {
        let progress = DisplayProgress::new("Test message".to_string(), ProgressCategory::Upload)
            .with_details("Additional info".to_string());

        assert_eq!(progress.details, Some("Additional info".to_string()));
    }

    #[test]
    fn test_from_upload_progress_validating_file() {
        let domain_progress = UploadProgress::new(UploadPhase::ValidatingFile {
            file_path: "/path/to/file.mp4".to_string(),
        });

        let display_progress = domain_progress
            .to_display()
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

        let display_progress = domain_progress
            .to_display()
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

        let display_progress = domain_progress
            .to_display()
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

        let display_progress = domain_progress
            .to_display()
            .expect("update should be displayed");

        assert_eq!(display_progress.message, "Still waiting... (20s elapsed)");
        assert_eq!(display_progress.category, ProgressCategory::Processing);
    }

    #[test]
    fn test_from_upload_progress_waiting_suppressed() {
        // 設定間隔未満の更新は表示を抑制（Noneを返す）
        let domain_progress = UploadProgress::new(UploadPhase::WaitingForAsset {
            upload_id: "test_id".to_string(),
            elapsed_secs: 5,
        });

        let display_progress = domain_progress.to_display();

        // None（表示抑制）が返されることを確認
        assert!(
            display_progress.is_none(),
            "Updates under configured interval should be suppressed"
        );
    }

    #[test]
    fn test_from_upload_progress_completed() {
        let domain_progress = UploadProgress::new(UploadPhase::Completed {
            asset_id: "asset_123".to_string(),
        });

        let display_progress = domain_progress
            .to_display()
            .expect("update should be displayed");

        assert_eq!(display_progress.message, "Asset created: asset_123");
        assert_eq!(display_progress.category, ProgressCategory::Completed);
    }
}
