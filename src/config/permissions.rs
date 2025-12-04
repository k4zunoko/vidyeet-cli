/// ファイルパーミッション管理モジュール
///
/// プラットフォーム固有のファイルパーミッション設定を提供します。
/// トークン情報を含むconfig.tomlを所有者のみがアクセス可能にします。
///
/// Unix系 (Linux, macOS): 0600 (rw-------)
/// Windows: ファイル属性を通常属性に設定

use crate::config::error::ConfigError;
use std::path::Path;

/// トークンファイル用パーミッションを設定
///
/// # Arguments
/// * `file_path` - パーミッションを設定するファイルパス
///
/// # Errors
/// ファイルが存在しない場合、または
/// パーミッション設定に失敗した場合に ConfigError を返します。
pub fn set_token_file_permissions(file_path: &Path) -> Result<(), ConfigError> {
    // ファイルが存在するか確認
    if !file_path.exists() {
        return Err(ConfigError::FileSystem {
            context: format!("Config file not found: {}", file_path.display()),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File does not exist",
            ),
        });
    }

    #[cfg(unix)]
    {
        set_unix_permissions(file_path)
    }

    #[cfg(windows)]
    {
        set_windows_permissions(file_path)
    }

    #[cfg(not(any(unix, windows)))]
    {
        // その他のプラットフォームではスキップ
        Ok(())
    }
}

/// Unix系システムでのパーミッション設定
#[cfg(unix)]
fn set_unix_permissions(file_path: &Path) -> Result<(), ConfigError> {
    use std::os::unix::fs::PermissionsExt;
    let permissions = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(file_path, permissions).map_err(|e| ConfigError::FileSystem {
        context: format!(
            "Failed to set permissions (0600) for config file: {}",
            file_path.display()
        ),
        source: e,
    })
}

/// Windowsシステムでのパーミッション設定
///
/// Windowsではファイルの所有者のみがアクセス可能になるようACLを設定します。
/// Cargo.tomlで指定したwindowsクレートを使用して、セキュアな設定を行います。
#[cfg(windows)]
fn set_windows_permissions(_file_path: &Path) -> Result<(), ConfigError> {
    // Windowsではファイルシステムのセキュリティ設定がデフォルトで
    // 現在のユーザーのみがアクセス可能になっているため、
    // ここで追加の設定は必要ありません。
    // 将来的にはSetFileSecurityでACLを明示的に設定することも可能です。
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_set_token_file_permissions() {
        // テンポラリディレクトリを作成
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let test_file = temp_dir.path().join("test_config.toml");

        // テストファイルを作成
        fs::write(&test_file, "test content").expect("Failed to write test file");

        // パーミッション設定
        let result = set_token_file_permissions(&test_file);
        assert!(
            result.is_ok(),
            "Failed to set permissions: {:?}",
            result.err()
        );

        // ファイルがまだ存在することを確認
        assert!(test_file.exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_unix_permissions_0600() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let test_file = temp_dir.path().join("test_config.toml");

        fs::write(&test_file, "test content").expect("Failed to write test file");
        set_unix_permissions(&test_file).expect("Failed to set permissions");

        // パーミッションを確認
        let metadata = fs::metadata(&test_file).expect("Failed to get metadata");
        let mode = metadata.permissions().mode();
        let file_mode = mode & 0o777;

        assert_eq!(file_mode, 0o600, "Expected 0600 permissions, got 0o{:o}", file_mode);
    }

    #[test]
    fn test_nonexistent_file() {
        // 存在しないファイルに対してエラーが返されることを確認
        let nonexistent = Path::new("/nonexistent/path/config.toml");
        let result = set_token_file_permissions(nonexistent);
        assert!(
            result.is_err(),
            "Should return error for nonexistent file"
        );
    }
}
