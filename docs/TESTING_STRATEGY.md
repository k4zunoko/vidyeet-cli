# テスト戦略

## 概要

vidyeet-cliのテスト戦略を定義します。このドキュメントでは、ユニットテスト、統合テスト、エンドツーエンドテストの方針、テスト実行方法、カバレッジ目標を説明します。

## テスト方針

### 基本原則

1. **レイヤーごとのテスト**
   - 各層（domain, config, api, commands, presentation）を独立してテスト
   - 外部依存をモック/スタブで置き換え

2. **Fail Fastの検証**
   - エラーが早期に検出されることを確認
   - 境界値でのバリデーションをテスト

3. **シングルスレッド実行**
   - ファイルシステムやグローバル状態を使用するため、並列実行を避ける
   - `cargo test -- --test-threads=1` で実行

4. **実際のAPI呼び出しは統合テストのみ**
   - ユニットテストではモックを使用
   - 統合テストでMux Sandbox環境を使用（手動実行）

## テスト階層

### 1. ユニットテスト

**対象:** 個別の関数・メソッドの動作

**配置:** 各モジュールの`#[cfg(test)] mod tests`ブロック内

**実行:**
```bash
cargo test
```

#### ドメイン層のテスト

**目的:** ビジネスルール・バリデーションの検証

```rust
// src/domain/validator.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_upload_file_success() {
        // 有効なファイルのテスト
        let result = validate_upload_file("test.mp4");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_upload_file_not_found() {
        // ファイルが存在しない場合
        let result = validate_upload_file("nonexistent.mp4");
        assert!(result.is_err());
        
        let err = result.unwrap_err();
        assert!(err.to_string().contains("File not found"));
    }

    #[test]
    fn test_validate_upload_file_too_large() {
        // ファイルサイズ超過のテスト
        // モックファイルを作成してテスト
    }

    #[test]
    fn test_validate_upload_file_unsupported_format() {
        // サポートされていない形式のテスト
        let result = validate_upload_file("test.txt");
        assert!(result.is_err());
        
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Unsupported format"));
    }
}
```

#### 設定層のテスト

**目的:** 設定ファイルの読み書き、バリデーションの検証

```rust
// src/config/user.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        // 保存
        let mut config = UserConfig::default();
        config.auth = Some(AuthConfig {
            token_id: "test_id".to_string(),
            token_secret: "test_secret".to_string(),
        });
        
        // 一時パスに保存する内部メソッドを使用
        config.save_to_path(&config_path).unwrap();
        
        // 読込
        let loaded = UserConfig::load_from_path(&config_path).unwrap();
        assert!(loaded.has_auth());
        assert_eq!(loaded.auth.unwrap().token_id, "test_id");
    }

    #[test]
    fn test_invalid_token_validation() {
        let config = UserConfig {
            auth: Some(AuthConfig {
                token_id: "".to_string(),
                token_secret: "secret".to_string(),
            }),
            timezone_offset_seconds: 0,
        };
        
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_default_config() {
        let config = UserConfig::default();
        assert!(!config.has_auth());
    }
}
```

#### API層のテスト

**目的:** HTTP通信、認証ヘッダー生成の検証

```rust
// src/api/auth.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_header_generation() {
        let manager = AuthManager::new(
            "my_token_id".to_string(),
            "my_token_secret".to_string()
        );
        
        let header = manager.get_auth_header();
        assert!(header.starts_with("Basic "));
        
        // Base64デコードして検証
        let encoded = header.strip_prefix("Basic ").unwrap();
        let decoded = base64::decode(encoded).unwrap();
        let decoded_str = String::from_utf8(decoded).unwrap();
        assert_eq!(decoded_str, "my_token_id:my_token_secret");
    }

    #[test]
    fn test_token_id_masking() {
        let manager = AuthManager::new(
            "abcdef123456789".to_string(),
            "secret".to_string()
        );
        
        let masked = manager.get_masked_token_id();
        assert_eq!(masked, "abc***789");
    }

    #[test]
    fn test_short_token_id_masking() {
        let manager = AuthManager::new(
            "short".to_string(),
            "secret".to_string()
        );
        
        let masked = manager.get_masked_token_id();
        assert_eq!(masked, "*****");
    }
}
```

#### エラーハンドリングのテスト

**目的:** 終了コード決定、エラーメッセージ生成の検証

```rust
// src/error_severity.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_codes() {
        assert_eq!(ErrorSeverity::UserError.exit_code(), 1);
        assert_eq!(ErrorSeverity::ConfigError.exit_code(), 2);
        assert_eq!(ErrorSeverity::SystemError.exit_code(), 3);
    }

    #[test]
    fn test_severity_equality() {
        assert_eq!(ErrorSeverity::UserError, ErrorSeverity::UserError);
        assert_ne!(ErrorSeverity::UserError, ErrorSeverity::ConfigError);
    }
}

// src/main.rs (ヘルパー関数のテスト)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_error_info_domain_error() {
        let err: anyhow::Error = DomainError::FileNotFound("test.mp4".to_string()).into();
        let err = err.context("Command failed");
        
        let (exit_code, hint) = extract_error_info(&err);
        assert_eq!(exit_code, 1);
        assert!(hint.is_some());
        assert!(hint.unwrap().contains("Check that the file path"));
    }

    #[test]
    fn test_extract_error_info_config_error() {
        let err: anyhow::Error = ConfigError::TokenNotFound.into();
        let err = err.context("Login required");
        
        let (exit_code, hint) = extract_error_info(&err);
        assert_eq!(exit_code, 2);
        assert!(hint.unwrap().contains("vidyeet login"));
    }

    #[test]
    fn test_extract_error_info_infra_error() {
        let err: anyhow::Error = InfraError::NetworkError("timeout".to_string()).into();
        let err = err.context("API call failed");
        
        let (exit_code, _) = extract_error_info(&err);
        assert_eq!(exit_code, 3);
    }
}
```

### 2. 統合テスト

**対象:** 複数モジュールの連携動作

**配置:** `tests/` ディレクトリ（将来実装）

**実行:**
```bash
cargo test --test integration_tests
```

#### コマンド実行のテスト

```rust
// tests/integration_tests.rs
use vidyeet_cli::commands;

#[tokio::test]
async fn test_upload_with_invalid_file() {
    let result = commands::upload::execute("nonexistent.mp4", None).await;
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    
    // エラーチェーンを検証
    let domain_err = err
        .chain()
        .find_map(|e| e.downcast_ref::<DomainError>())
        .expect("Should contain DomainError");
    
    match domain_err {
        DomainError::FileNotFound(_) => {}, // OK
        _ => panic!("Expected FileNotFound error"),
    }
}

#[tokio::test]
async fn test_upload_without_authentication() {
    // 認証情報を削除した状態でアップロードを試行
    // ConfigErrorが発生することを確認
}
```

### 3. エンドツーエンドテスト

**対象:** 実際のMux APIとの通信

**実行:** 手動実行（Mux Sandbox環境を使用）

**手順:**

1. **テスト用Access Token作成**
   - Mux Dashboardでテスト用トークンを作成
   - 環境変数に設定: `MUX_TOKEN_ID`, `MUX_TOKEN_SECRET`

2. **テストシナリオ実行**

```bash
# 1. ログイン
echo "$MUX_TOKEN_ID\n$MUX_TOKEN_SECRET" | vidyeet login --stdin

# 2. ステータス確認
vidyeet status
# 期待: "✓ Authenticated"

# 3. アップロード
vidyeet upload test_video.mp4
# 期待: Asset ID, HLS URL, MP4 URL が表示される

# 4. 動画一覧
vidyeet list
# 期待: アップロードした動画が含まれる

# 5. 動画詳細
vidyeet show <asset_id>
# 期待: 詳細情報が表示される

# 6. 動画削除
vidyeet delete <asset_id> --force
# 期待: "✓ Asset deleted successfully."

# 7. ログアウト
vidyeet logout
# 期待: "✓ Logged out successfully."
```

3. **機械可読出力のテスト**

```bash
# JSON出力の検証
vidyeet --machine status | jq .
# 期待: 有効なJSON

vidyeet --machine list | jq '.data | length'
# 期待: 数値

# 進捗通知のテスト
vidyeet --machine upload test_video.mp4 --progress > output.jsonl
# 期待: JSONL形式で各行が有効なJSON
```

## テスト実行

### ローカル実行

```bash
# すべてのテスト（シングルスレッド）
cargo test -- --test-threads=1

# 特定のモジュール
cargo test config::tests -- --test-threads=1

# 特定のテスト
cargo test test_validate_upload_file_success -- --test-threads=1

# 詳細ログ付き
RUST_LOG=debug cargo test -- --test-threads=1 --nocapture
```

### CI/CD実行

```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run tests
        run: cargo test -- --test-threads=1
      - name: Run clippy
        run: cargo clippy -- -D warnings
```

## テストカバレッジ目標

### レイヤーごとの目標

| レイヤー | カバレッジ目標 | 理由 |
|---------|--------------|------|
| ドメイン層 | 90%+ | ビジネスロジックの中核 |
| 設定層 | 80%+ | 設定管理は重要だが、ファイルI/Oが多い |
| API層 | 60%+ | 外部API依存が多く、モックが複雑 |
| コマンド層 | 70%+ | 統合テストでカバー |
| プレゼンテーション層 | 50%+ | UI/出力フォーマットの変更が多い |

### カバレッジ測定

```bash
# tarpaulinを使用
cargo install cargo-tarpaulin
cargo tarpaulin --out Html -- --test-threads=1

# llvm-covを使用
cargo install cargo-llvm-cov
cargo llvm-cov --html -- --test-threads=1
```

## モック/スタブ戦略

### ファイルシステムのモック

```rust
// tempfileクレートを使用
use tempfile::TempDir;

#[test]
fn test_file_operations() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.mp4");
    
    // テスト用ファイル作成
    std::fs::write(&file_path, b"test data").unwrap();
    
    // テスト実行
    let result = validate_upload_file(file_path.to_str().unwrap());
    assert!(result.is_ok());
    
    // TempDirがドロップされると自動削除
}
```

### HTTP通信のモック

```rust
// wiremockクレートを使用（将来実装）
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_api_create_direct_upload() {
    // モックサーバー起動
    let mock_server = MockServer::start().await;
    
    // モックレスポンス設定
    Mock::given(method("POST"))
        .and(path("/video/v1/uploads"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "data": {
                "id": "upload_test123",
                "url": "https://storage.example.com/upload",
                "status": "waiting"
            }
        })))
        .mount(&mock_server)
        .await;
    
    // クライアント作成（モックサーバーのURLを使用）
    let client = MuxClient::new_with_base_url(
        "test_id".to_string(),
        "test_secret".to_string(),
        mock_server.uri(),
    );
    
    // テスト実行
    let result = client.create_direct_upload().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().id, "upload_test123");
}
```

## テスト命名規則

### パターン

```rust
#[test]
fn test_<function_name>_<scenario>() {
    // test_validate_upload_file_when_file_not_found
    // test_extract_error_info_returns_domain_error_code
    // test_save_config_creates_parent_directories
}
```

### 良い例

- `test_validate_upload_file_success`
- `test_validate_upload_file_not_found`
- `test_validate_upload_file_too_large`
- `test_auth_header_generation`
- `test_token_id_masking`

### 悪い例

- `test1`, `test2` （何をテストしているか不明）
- `test_function` （シナリオが不明）
- `it_works` （一般的すぎる）

## 継続的な改善

### テスト追加のタイミング

1. **新機能追加時**: 必ずテストを追加
2. **バグ修正時**: バグを再現するテストを追加
3. **リファクタリング時**: 既存テストが通ることを確認
4. **エッジケース発見時**: 境界値テストを追加

### コードレビューでのチェック項目

- [ ] 新しいコードにテストが含まれているか
- [ ] テストが適切なシナリオをカバーしているか
- [ ] テストが独立して実行できるか（外部状態に依存していないか）
- [ ] テスト名が命名規則に従っているか
- [ ] モック/スタブが適切に使用されているか

## 既知の制約とトレードオフ

### シングルスレッド実行

**理由:**
- 設定ファイル（`config.toml`）への読み書きが競合する
- 一時ファイル/ディレクトリの使用

**トレードオフ:**
- テスト実行時間が長くなる
- CI/CDでの並列実行ができない

**将来の改善:**
- テストごとに独立した設定ファイルパスを使用
- 環境変数でパスをオーバーライド可能にする

### 実際のAPI呼び出し

**理由:**
- Mux APIのモック実装が複雑
- APIレスポンスの完全性を保証する必要がある

**トレードオフ:**
- E2Eテストは手動実行が必要
- CI/CDで自動化できない

**将来の改善:**
- Mux Sandbox環境用のCI/CD設定
- モックサーバーの充実化

## 参考資料

- [Rust Testing - The Rust Book](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [tokio Testing](https://tokio.rs/tokio/topics/testing)
- [wiremock documentation](https://docs.rs/wiremock/)
- [tempfile documentation](https://docs.rs/tempfile/)
- [Test-Driven Development in Rust](https://rust-lang.github.io/async-book/07_workarounds/05_async_in_traits.html)