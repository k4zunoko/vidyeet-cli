use serde::Serialize;
/// ドメイン層: アップロード進捗イベント定義
///
/// アップロード処理の各段階をビジネスロジックのイベントとして表現します。
/// プレゼンテーション層はこれらのイベントを受け取り、
/// 人間向けの進捗表示や機械向けの制御に使用します。
use std::time::SystemTime;

/// アップロード処理の各段階を表すイベント
///
/// # 設計意図
/// - ビジネスロジック（処理フロー）の可視化
/// - プレゼンテーション層での柔軟な出力制御
/// - 将来のプログレスバー実装への拡張性
/// - 機械可読出力のためにSerialize可能
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum UploadPhase {
    /// ファイル検証開始
    ValidatingFile { file_path: String },

    /// ファイル検証完了
    FileValidated {
        file_name: String,
        size_bytes: u64,
        format: String,
    },

    /// Direct Upload URL作成中
    CreatingDirectUpload { file_name: String },

    /// Direct Upload作成完了
    DirectUploadCreated { upload_id: String },

    /// ファイルアップロード開始
    UploadingFile {
        file_name: String,
        size_bytes: u64,
        total_chunks: usize,
    },

    /// チャンクアップロード中
    UploadingChunk {
        current_chunk: usize,
        total_chunks: usize,
        bytes_sent: u64,
        total_bytes: u64,
    },

    /// ファイルアップロード完了
    FileUploaded { file_name: String, size_bytes: u64 },

    /// アセット作成完了を待機中
    WaitingForAsset {
        #[allow(dead_code)]
        upload_id: String,
        elapsed_secs: u64,
    },

    /// アップロード処理完了
    Completed { asset_id: String },
}

/// アップロード進捗情報
///
/// 各処理段階のイベントとタイムスタンプを保持します。
#[derive(Debug, Clone, Serialize)]
pub struct UploadProgress {
    /// 処理段階
    pub phase: UploadPhase,
    /// イベント発生時刻（将来の分析や詳細ログ用に保持）
    #[serde(skip)]
    #[allow(dead_code)]
    pub timestamp: SystemTime,
}

impl UploadProgress {
    /// 新しい進捗情報を作成
    pub fn new(phase: UploadPhase) -> Self {
        Self {
            phase,
            timestamp: SystemTime::now(),
        }
    }
}
