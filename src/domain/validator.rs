/// ドメインサービス: ファイルバリデーション
///
/// アップロード対象のファイルを検証する。
/// ドメイン層の責務として、ビジネスルールを適用する。
///
/// 設定値（最大ファイルサイズ、サポート形式）はAPP_CONFIGから取得します。
use crate::config::APP_CONFIG;
use crate::domain::error::DomainError;
use std::path::Path;

/// ファイルのバリデーション結果
pub struct ValidationResult {
    pub path: String,
    pub size: u64,
    pub extension: String,
}

/// アップロード対象のファイルをバリデーションする
///
/// # エラー
/// - ファイルが存在しない
/// - ディレクトリが指定された
/// - ファイルが空
/// - サポートされていない形式
/// - ファイルサイズが制限を超過
pub fn validate_upload_file(file_path: &str) -> Result<ValidationResult, DomainError> {
    let path = Path::new(file_path);

    // 存在確認
    if !path.exists() {
        return Err(DomainError::FileNotFound {
            path: file_path.to_string(),
        });
    }

    // メタデータ取得（InfraErrorに変換せず、ここではDomainErrorとして扱う）
    let metadata = std::fs::metadata(path).map_err(|_| DomainError::FileNotFound {
        path: file_path.to_string(),
    })?;

    // ディレクトリチェック
    if metadata.is_dir() {
        return Err(DomainError::NotAFile {
            path: file_path.to_string(),
        });
    }

    // 空ファイルチェック
    let size = metadata.len();
    if size == 0 {
        return Err(DomainError::EmptyFile {
            path: file_path.to_string(),
        });
    }

    // ファイルサイズチェック（APP_CONFIGから設定値を取得）
    let max_file_size = APP_CONFIG.upload.max_file_size;
    if size > max_file_size {
        return Err(DomainError::FileTooLarge {
            size,
            max: max_file_size,
        });
    }

    // 拡張子チェック（APP_CONFIGから設定値を取得）
    let supported_formats = APP_CONFIG.upload.supported_formats;
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
        .ok_or_else(|| DomainError::InvalidFormat {
            path: file_path.to_string(),
            expected: format!("one of: {}", supported_formats.join(", ")),
            found: "no extension".to_string(),
        })?;

    if !supported_formats.contains(&extension.as_str()) {
        return Err(DomainError::InvalidFormat {
            path: file_path.to_string(),
            expected: format!("one of: {}", supported_formats.join(", ")),
            found: extension.clone(),
        });
    }

    Ok(ValidationResult {
        path: file_path.to_string(),
        size,
        extension,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_formats() {
        // APP_CONFIGから取得した形式リストのテスト
        let formats = APP_CONFIG.upload.supported_formats;
        assert!(formats.contains(&"mp4"));
        assert!(formats.contains(&"mov"));
        assert!(formats.contains(&"webm"));
    }
}
