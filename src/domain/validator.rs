/// ドメインサービス: ファイルバリデーション
///
/// アップロード対象のファイルを検証する。
/// ドメイン層の責務として、ビジネスルールを適用する。
///
/// 設定値（最大ファイルサイズ、サポート形式）はAPP_CONFIGから取得します。
use crate::config::APP_CONFIG;
use crate::domain::error::DomainError;
use std::path::Path;

/// バリデーション結果の型エイリアス
type ValidationResult<T> = Result<T, DomainError>;

/// ファイルのバリデーション結果
#[derive(Debug, Clone)]
pub struct FileValidation {
    pub path: String,
    pub size: u64,
    pub extension: String,
}

/// アップロード対象のファイルをバリデーションする
///
/// # 引数
/// * `file_path` - 検証対象のファイルパス
///
/// # 戻り値
/// 検証に成功した場合は`FileValidation`を返す
///
/// # エラー
/// - ファイルが存在しない
/// - ディレクトリが指定された
/// - ファイルが空
/// - サポートされていない形式
/// - ファイルサイズが制限を超過
pub fn validate_upload_file(file_path: &str) -> ValidationResult<FileValidation> {
    let path = Path::new(file_path);

    // 存在確認
    if !path.exists() {
        return Err(DomainError::file_not_found(file_path));
    }

    // メタデータ取得
    let metadata = std::fs::metadata(path).map_err(|_| DomainError::file_not_found(file_path))?;

    // ディレクトリチェック
    if metadata.is_dir() {
        return Err(DomainError::not_a_file(file_path));
    }

    // 空ファイルチェック
    let size = metadata.len();
    if size == 0 {
        return Err(DomainError::empty_file(file_path));
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
    let extension = extract_extension(path, file_path, supported_formats)?;

    if !supported_formats.contains(&extension.as_str()) {
        return Err(DomainError::invalid_format(
            file_path,
            supported_formats,
            &extension,
        ));
    }

    Ok(FileValidation {
        path: file_path.to_string(),
        size,
        extension,
    })
}

/// ファイルパスから拡張子を抽出する
fn extract_extension(
    path: &Path,
    file_path: &str,
    supported_formats: &[&str],
) -> ValidationResult<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
        .ok_or_else(|| DomainError::invalid_format(file_path, supported_formats, "no extension"))
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
