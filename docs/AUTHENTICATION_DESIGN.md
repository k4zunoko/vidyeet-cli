# 認証フロー設計書

## 概要

vidyeet-cliは、Mux Video APIのHTTP Basic認証を使用したシンプルで安全な認証方式を採用しています。

### 設計方針

1. **Access Token IDとSecretをconfig.tomlに保存** - 環境変数より管理しやすい設定ファイルに保存
2. **ファイルパーミッションで保護** - config.tomlを0600(所有者のみ読み書き可能)に設定
3. **Gitignoreで保護** - config.tomlはバージョン管理から除外
4. **HTTP Basic認証** - Mux APIの標準認証方式(Access Token ID: username, Secret: password)

## 認証フロー

### 1. 初回ログイン（`login`コマンド）

```
┌──────────────┐
│ ユーザー      │
└──────┬───────┘
       │ vidyeet-cli login
       ↓
┌──────────────────────────┐
│ 1. Access Token ID 入力  │
│   対話入力 （エコーオフ）  │
└──────┬───────────────────┘
       │
       ↓
┌──────────────────────────┐
│ 2. Access Token Secret入力│
│   対話入力（rpassword crate使用）│
└──────┬───────────────────┘
       │
       ↓
┌──────────────────────────┐
│ 3. 認証テスト            │
│    GET /video/v1/assets  │
│    Authorization: Basic  │
│    (base64(ID:Secret))   │
└──────┬───────────────────┘
       │
       ↓ 成功
┌──────────────────────────┐
│ 4. config.tomlに保存     │
│    ~/.config/vidyeet-cli/│
│    config.toml           │
│    [auth]                │
│    token_id = "..."      │
│    token_secret = "..."  │
│    (パーミッション: 0600) │
└──────┬───────────────────┘
       │
       ↓
┌──────────────┐
│ ✓ ログイン完了│
└──────────────┘
```

### 2. コマンド実行時（`upload` / `list` / `delete`）

```
┌──────────────┐
│ ユーザー      │
└──────┬───────┘
       │ vidyeet-cli upload video.mp4
       ↓
┌──────────────────────────┐
│ 1. config.toml 読込      │
│    [auth]                │
│    token_id = "..."      │
│    token_secret = "..."  │
└──────┬───────────────────┘
       │
       ↓
┌──────────────────────────┐
│ 2. 認証情報確認          │
│    - token_idが存在?     │
│    - token_secretが存在? │
│    → NO: "Please login"  │
│    → YES: 次へ           │
└──────┬───────────────────┘
       │
       ↓
┌──────────────────────────┐
│ 3. HTTP Basic認証で      │
│    APIリクエスト実行      │
│    Authorization: Basic  │
│    base64(token_id:      │
│           token_secret)  │
└──────┬───────────────────┘
       │
       ↓
┌──────────────┐
│ ✓ 処理完了    │
└──────────────┘
```

### 3. ログアウト（`logout`コマンド）

```
┌──────────────┐
│ ユーザー      │
└──────┬───────┘
       │ vidyeet-cli logout
       ↓
┌──────────────────────────┐
│ 1. 認証情報削除          │
│    config.toml から      │
│    [auth] セクション削除  │
│    または               │
│    token_id, token_secret│
│    の値を削除            │
└──────┬───────────────────┘
       │
       ↓
┌──────────────┐
│ ✓ ログアウト完了│
└──────────────┘
```

## ファイル構造

### トークンストレージ

**パス**: `~/.config/vidyeet-cli/config.toml`

**フォーマット**:
```toml
# ユーザー設定
default_title = "My Video"
auto_copy_url = true
show_notification = true

# 認証情報（ログイン後に自動追加）
[auth]
refresh_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

**保存される情報**:
- ✅ **refresh_token**: リフレッシュトークンのみ（長期間有効）
- ❌ **access_token**: 保存しない（メモリのみ、1時間で失効）
- ❌ **api_key**: 保存しない（ログイン時のみメモリで使用）

**パーミッション** (Unix系):
- `0600` (rw-------) - 所有者のみ読み書き可能
- ディレクトリ: `0700` (rwx------)

## セキュリティ考慮事項

## ファイル構造

### トークンストレージ

**パス**: `~/.config/vidyeet-cli/config.toml`

**フォーマット**:
```toml
# ユーザー設定
default_title = "My Video"
auto_copy_url = true
show_notification = true

# 認証情報（ログイン後に自動追加）
[auth]
token_id = "your-access-token-id"
token_secret = "your-access-token-secret"
```

**パーミッション**:
- Unix系: `0600` (所有者のみ読み書き可能)
- Windows: ACLで所有者のみアクセス可能に設定

### Access Token vs Secret

| 項目 | Access Token ID | Access Token Secret |
|------|----------------|---------------------|
| **用途** | HTTP Basic認証のユーザー名 | HTTP Basic認証のパスワード |
| **取得方法** | Muxダッシュボード | Muxダッシュボード(作成時のみ表示) |
| **再表示** | ✅ 可能 | ❌ 不可能(ハッシュのみ保存) |
| **漏洩時の影響** | ⚠️ Secretと組み合わせると重大 | ⚠️ 重大(すぐに無効化必要) |
| **永続化** | ✅ config.tomlに保存 | ✅ config.tomlに保存 |

### セキュリティ対策

1. **Access Tokenの保護**
   - ✅ config.tomlに保存（パーミッション0600）
   - ✅ .gitignoreに追加
   - ✅ ログ出力禁止
   - ✅ エラーメッセージに含めない

2. **トークンファイルの保護**
   - ✅ パーミッション制限（0600）
   - ✅ ホームディレクトリ内（他ユーザーアクセス不可）
   - ⚠️ 暗号化（将来の拡張で検討）
   - ⚠️ OS keyring統合（将来の拡張で検討）

3. **通信の保護**
   - ✅ HTTPS必須（Mux API標準）
   - ✅ 証明書検証（reqwestデフォルト）
   - ✅ タイムアウト設定

4. **トークン管理**
   - ✅ 認証失敗時に再ログイン促す
   - ✅ トークン値をログ/エラーに含めない
   - ✅ マスキング処理（表示時は `*****` に置換）

## エラーハンドリング

### 認証関連エラー

| エラー | 原因 | 対処 |
|-------|------|------|
| `TokenNotFound` | config.tomlに[auth]セクションがない | `vidyeet-cli login` 実行を促す |
| `InvalidCredentials` | Token IDまたはSecretが間違っている | `vidyeet-cli login` 再実行を促す |
| `401 Unauthorized` | トークンが無効化されている | `vidyeet-cli login` 再実行を促す |
| `ConfigFileError` | config.tomlの読み込みに失敗 | ファイルの存在とパーミッション確認 |

### エラーメッセージ例

```
Error: Token not found
No authentication credentials found. Please run 'vidyeet-cli login' first.

Hint: You need to authenticate with your Mux Access Token.
      Get your token at: https://dashboard.mux.com/settings/access-tokens
      Run: vidyeet-cli login
```

```
Error: Invalid credentials
Authentication failed. Please check your Access Token ID and Secret.

Hint: Make sure you copied both the Token ID and Secret correctly.
      Run: vidyeet-cli login
```

## コマンド仕様

### `login` コマンド

**用途**: Mux Access Tokenで認証し、認証情報を保存

**構文**:
```bash
vidyeet-cli login [--token-id <id>] [--token-secret <secret>]
```

**オプション**:
- `--token-id <id>`: Access Token IDを指定（省略時は対話的入力）
- `--token-secret <secret>`: Access Token Secretを指定（省略時は対話的入力）

**例**:
```bash
# 対話的入力（推奨）
$ vidyeet-cli login
Enter your Mux Access Token ID: abc123xyz
Enter your Mux Access Token Secret: ****
Authenticating...
✓ Successfully logged in!
Your credentials have been saved.

# コマンドライン引数（非推奨: シェル履歴に残る）
$ vidyeet-cli login --token-id abc123 --token-secret secret456
Authenticating...
✓ Successfully logged in!
```

### `logout` コマンド

**用途**: 保存された認証情報を削除

**構文**:
```bash
vidyeet-cli logout
```

**例**:
```bash
$ vidyeet-cli logout
Logging out...
✓ Successfully logged out!
Your credentials have been removed.
```

## 実装モジュール

### `config/user.rs`

**責務**: ユーザー設定とAccess Token管理

**構造**:
```rust
pub struct UserConfig {
    pub default_title: Option<String>,
    pub auto_copy_url: bool,
    pub show_notification: bool,
    pub auth: Option<AuthConfig>,  // 認証情報
}

pub struct AuthConfig {
    pub token_id: String,      // Access Token ID
    pub token_secret: String,  // Access Token Secret
}
```

**主要API**:
- `UserConfig::load()` - config.toml読み込み
- `UserConfig::save()` - config.toml保存
- `UserConfig::set_auth()` - 認証情報設定
- `UserConfig::has_auth()` - 認証情報存在確認
- `UserConfig::clear_auth()` - 認証情報削除

### `api/auth.rs`

**責務**: Mux API認証処理

**構造**:
```rust
pub struct AuthManager {
    token_id: String,
    token_secret: String,
}
```

**主要API**:
- `AuthManager::new(token_id, token_secret)` - 初期化
- `AuthManager::from_config()` - UserConfigから初期化
- `AuthManager::test_credentials()` - 認証情報をテスト（GET /video/v1/assets で確認）
- `AuthManager::get_auth_header()` - HTTP Basic認証ヘッダー生成

### `api/client.rs`

**責務**: Mux API HTTPクライアント

**構造**:
```rust
pub struct MuxClient {
    client: reqwest::Client,
    base_url: String,
    auth: AuthManager,
}
```

**主要API**:
- `MuxClient::new(auth)` - 初期化
- `MuxClient::create_direct_upload()` - Direct Upload作成
- `MuxClient::list_assets()` - アセット一覧取得
- `MuxClient::get_asset()` - アセット情報取得
- `MuxClient::delete_asset()` - アセット削除
- `MuxClient::get_upload_status()` - Upload状態確認

### `commands/login.rs`

**責務**: loginコマンドの実装

**処理フロー**:
1. Token IDとSecret取得（引数 or プロンプト）
2. `AuthManager::test_credentials()` で認証テスト
3. 成功したら`UserConfig::set_auth()` で保存
4. 成功メッセージ表示

### `commands/logout.rs`

**責務**: logoutコマンドの実装

**処理フロー**:
1. `UserConfig::clear_auth()` 実行
2. 成功メッセージ表示

### `commands/upload.rs`

**責務**: uploadコマンドの実装

**処理フロー**:
1. `UserConfig::load()` で認証情報読み込み
2. 認証情報がなければエラー（login促す）
3. `MuxClient::new()` で初期化
4. `MuxClient::create_direct_upload()` でUpload URL取得
5. 動画ファイルをPUTリクエストでアップロード
6. `MuxClient::get_upload_status()` で完了確認
7. Asset IDと再生URLを表示

## 無料枠制限への対応

### 自動古いアセット削除機能

Mux無料プランでは最大10個の動画しかアップロードできないため、アップロード失敗時に自動的に古いアセットを削除します。

**実装方針**:

1. **アップロード前チェック**
   - `MuxClient::list_assets()` でアセット数を確認
   - 10個以上ある場合は警告表示

2. **エラー時の自動削除**
   - アップロード失敗時にエラーコードを確認
   - クォータエラーの場合は自動削除モードに移行
   - `created_at` でソートして最も古いアセットを特定
   - 確認プロンプト表示後に削除
   - 削除後に再アップロード

3. **削除ロジック**
```rust
pub async fn ensure_upload_capacity(client: &MuxClient) -> Result<()> {
    let assets = client.list_assets().await?;
    
    if assets.len() >= 10 {
        println!("Warning: You have {} assets (max 10 in free plan)", assets.len());
        
        // created_at でソート（古い順）
        let mut sorted = assets;
        sorted.sort_by_key(|a| a.created_at);
        
        let oldest = &sorted[0];
        println!("Oldest asset: {} (created: {})", oldest.id, oldest.created_at);
        
        let confirm = prompt_yes_no("Delete oldest asset to make room?")?;
        if confirm {
            client.delete_asset(&oldest.id).await?;
            println!("✓ Deleted asset: {}", oldest.id);
        } else {
            return Err(Error::QuotaExceeded);
        }
    }
    
    Ok(())
}
```

## テスト戦略

### ユニットテスト

1. **UserConfig**
   - 認証情報の保存/読み込み
   - パーミッション設定（Unix）
   - TOML形式の正確性

2. **AuthManager**
   - HTTP Basic認証ヘッダー生成
   - エラーハンドリング

### 統合テスト

1. **Mockサーバー** (wiremock)
   - `/video/v1/assets` モック（認証テスト用）
   - `/video/v1/uploads` モック
   - エラーレスポンステスト

### エンドツーエンドテスト

1. **手動テスト** (Mux Sandbox環境)
   - login → upload → logout フロー
   - 認証エラー時の動作
   - 無効なトークンでの動作

## 将来の拡張

### トークン暗号化

**目的**: ファイル漏洩時のリスク軽減

**実装案**:
- OS keyringを使用した暗号化キー管理
- AES-256-GCMでトークン暗号化
- クレート: `keyring`, `aes-gcm`

### OS keyring統合

**目的**: よりセキュアなトークン保存

**実装案**:
- macOS: Keychain
- Windows: Credential Manager
- Linux: Secret Service API
- クレート: `keyring`

### 環境変数サポート

**目的**: CI/CD環境での利用

**実装案**:
- `MUX_TOKEN_ID` 環境変数
- `MUX_TOKEN_SECRET` 環境変数
- config.tomlより優先度を高く設定

### 複数アカウント対応

**目的**: 複数のMuxアカウント管理

**実装案**:
- プロファイル機能の実装
- `vidyeet-cli login --profile production`
- `vidyeet-cli upload --profile staging video.mp4`
- config.tomlに複数の認証情報を保存

## 参考資料

- [Mux Authentication Guide](https://docs.mux.com/guides/make-api-requests)
- [Mux API Reference](https://docs.mux.com/api-reference)
- [Mux Direct Uploads](https://docs.mux.com/guides/upload-files-directly)
- [HTTP Basic Authentication - RFC 7617](https://datatracker.ietf.org/doc/html/rfc7617)
- [OWASP - Secure Token Storage](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)

---

**Last Updated**: 2025-11-27  
**Version**: 2.0  
**Status**: Mux Video対応版 - 設計完了
