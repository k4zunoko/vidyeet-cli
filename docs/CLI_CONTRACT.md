# CLI契約仕様

## 概要

vidyeet-cliのコマンドラインインターフェース（CLI）の完全な仕様を定義します。このドキュメントは、人間ユーザーとプログラム（スクリプト、CI/CD）の両方を対象としています。

## CLI設計原則

### UNIX哲学の実践

1. **stdout/stderrの明確な分離**
   - **stdout**: 機械可読データ（`--machine`フラグ指定時のみ）
   - **stderr**: 人間向けメッセージ（進捗、エラー、結果）
   - **終了コード**: 成否を示す（0=成功、1/2/3=エラー）

2. **パイプライン対応**
   - 標準入力からの認証情報入力（`--stdin`）
   - JSON出力をパイプライン処理可能

3. **明示的な制御**
   - TTY自動検出ではなく、`--machine`フラグで明示的に出力形式を指定

### 設計判断の根拠

**なぜTTY自動検出を使わないのか:**

```rust
// 悪い例（自動切り替え）
if atty::is(Stream::Stdout) {
    // 人間向け出力
} else {
    // 機械向けJSON出力
}
```

- **問題点**: 異なるTTY環境で予期しない動作変更が発生
- **解決策**: `--machine`フラグで明示的に制御

```rust
// 良い例（明示的制御）
if machine_output {
    println!("{}", json); // stdout
} else {
    eprintln!("✓ Success!"); // stderr
}
```

## 終了コード

| コード | 分類 | 説明 | 例 |
|-------|------|------|---|
| `0` | 成功 | コマンドが正常に完了 | - |
| `1` | ユーザーエラー | ユーザー入力や操作の問題 | ファイル不正、形式無効 |
| `2` | 設定エラー | 認証情報や設定の問題 | 未ログイン、トークン無効 |
| `3` | システムエラー | ネットワークやAPI側の問題 | API接続失敗、I/O障害 |

### 終了コードの活用例

**PowerShell:**
```powershell
vidyeet upload video.mp4
if ($LASTEXITCODE -eq 0) {
    Write-Host "成功"
} elseif ($LASTEXITCODE -eq 1) {
    Write-Host "ファイルエラー"
} elseif ($LASTEXITCODE -eq 2) {
    Write-Host "ログインが必要"
} else {
    Write-Host "システムエラー"
}
```

**Bash:**
```bash
vidyeet upload video.mp4
EXIT_CODE=$?
if [ $EXIT_CODE -eq 0 ]; then
    echo "成功"
elif [ $EXIT_CODE -eq 2 ]; then
    echo "ログインが必要"
    exit 1
fi
```

## グローバルフラグ

### --machine

機械可読なJSON形式で出力します。

**構文:**
```
vidyeet --machine <command> [args...]
```

**重要:** `--machine`は必ずコマンド名の**前**に指定してください。

**効果:**
- stdout に構造化JSONを出力
- エラーも JSON形式で出力
- 人間向けメッセージ（進捗表示など）は出力されない

## コマンド一覧

### login - ログイン

Mux APIの認証情報を設定します。

**構文:**
```bash
# 対話形式（推奨）
vidyeet login

# 標準入力から（CI/CD向け）
echo "$TOKEN_ID\n$TOKEN_SECRET" | vidyeet login --stdin
```

**フラグ:**
- `--stdin`: 標準入力から認証情報を読み込む（2行: Token ID, Token Secret）

**人間向け出力例（stderr）:**
```
Logging in to Mux Video...

Please enter your Mux Access Token credentials.
You can find them at: https://dashboard.mux.com/settings/access-tokens

Access Token ID: ████████
Access Token Secret: ****

✓ Login successful.
Authentication credentials have been saved.
```

**機械向け出力例（stdout、--machine）:**
```json
{
  "success": true,
  "command": "login",
  "was_logged_in": false,
  "action": "created"
}
```

**フィールド:**
- `success` (boolean): 常に`true`
- `command` (string): "login"
- `was_logged_in` (boolean): 既にログイン済みだった場合`true`
- `action` (string): "created"（新規）または"updated"（上書き）

**終了コード:**
- `0`: 成功
- `2`: 認証失敗
- `3`: ネットワークエラー

---

### logout - ログアウト

保存された認証情報を削除します。

**構文:**
```bash
vidyeet logout
```

**人間向け出力例（stderr）:**
```
✓ Logged out successfully.
Authentication credentials have been removed.
```

**機械向け出力例（stdout、--machine）:**
```json
{
  "success": true,
  "command": "logout",
  "was_logged_in": true
}
```

**フィールド:**
- `success` (boolean): 常に`true`
- `command` (string): "logout"
- `was_logged_in` (boolean): ログイン状態だった場合`true`

**終了コード:**
- `0`: 成功（ログイン状態でなくても成功）

---

### status - ステータス確認

現在の認証状態を確認します。

**構文:**
```bash
vidyeet status
```

**人間向け出力例（stderr、認証済み）:**
```
✓ Authenticated
Token ID: abc***xyz
```

**人間向け出力例（stderr、未認証）:**
```
✗ Not authenticated
Please run 'vidyeet login' to authenticate.
```

**機械向け出力例（stdout、--machine、認証済み）:**
```json
{
  "success": true,
  "command": "status",
  "is_authenticated": true,
  "token_id": "abc***xyz"
}
```

**機械向け出力例（stdout、--machine、未認証）:**
```json
{
  "success": true,
  "command": "status",
  "is_authenticated": false,
  "token_id": null
}
```

**フィールド:**
- `success` (boolean): 常に`true`
- `command` (string): "status"
- `is_authenticated` (boolean): 認証済みの場合`true`
- `token_id` (string | null): マスキングされたToken ID

**終了コード:**
- `0`: 成功（認証状態に関わらず）

---

### list - 動画一覧取得

アップロード済みの動画一覧を取得します。

**構文:**
```bash
vidyeet list
```

**人間向け出力例（stderr）:**
```
Videos (3 total):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
1. Asset ID: abc123xyz
   Status: ready
   Duration: 5:23
   Created: 2024-01-15 14:30:00 +09:00
   HLS URL: https://stream.mux.com/xyz789.m3u8

2. Asset ID: def456uvw
   Status: ready
   Duration: 10:45
   Created: 2024-01-14 09:15:00 +09:00
   HLS URL: https://stream.mux.com/uvw123.m3u8
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**機械向け出力例（stdout、--machine）:**
```json
{
  "success": true,
  "command": "list",
  "data": [
    {
      "id": "abc123xyz",
      "status": "ready",
      "playback_ids": [
        {
          "id": "xyz789",
          "policy": "public"
        }
      ],
      "duration": 323.5,
      "created_at": "1705296600",
      "aspect_ratio": "16:9",
      "video_quality": "basic",
      "resolution_tier": "1080p",
      "encoding_tier": "baseline",
      "tracks": [
        {
          "type": "video",
          "id": "track_video_001",
          "duration": 323.4,
          "max_width": 1920,
          "max_height": 1080,
          "max_frame_rate": 30.0
        },
        {
          "type": "audio",
          "id": "track_audio_001",
          "duration": 323.5,
          "max_channels": 2,
          "max_channel_layout": "stereo"
        }
      ],
      "static_renditions": {
        "files": [
          {
            "id": "rendition_001",
            "type": "mp4",
            "status": "ready",
            "resolution": "1080p",
            "name": "high.mp4",
            "ext": "mp4"
          }
        ]
      }
    }
  ],
  "total_count": 3
}
```

**フィールド:**
- `success` (boolean): 常に`true`
- `command` (string): "list"
- `data` (array): アセットデータの配列（Mux API完全レスポンス）
- `total_count` (number): 総アセット数

**終了コード:**
- `0`: 成功
- `2`: 未認証
- `3`: API通信エラー

---

### show - 動画詳細表示

指定したアセットIDの詳細情報を表示します。

**構文:**
```bash
vidyeet show <asset_id>
```

**引数:**
- `asset_id`: アセットID（必須）

**人間向け出力例（stderr）:**
```
Asset Details:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Asset ID:       abc123xyz
Status:         ready
Duration:       5:23 (323.5s)
Aspect Ratio:   16:9
Video Quality:  basic
Created At:     2024-01-15 14:30:00 +09:00

Playback Information:
--------------------
Playback ID #1: xyz789
  Policy:       public
HLS URL:        https://stream.mux.com/xyz789.m3u8
MP4 URL:        https://stream.mux.com/xyz789/high.mp4

Tracks:
-------
Track #1: video (duration: 323.4s)
  Resolution: 1920x1080
  Frame Rate: 30.0 fps
Track #2: audio (duration: 323.5s)
  Channels: 2 (stereo)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**機械向け出力例（stdout、--machine）:**
```json
{
  "success": true,
  "command": "show",
  "data": {
    "id": "abc123xyz",
    "status": "ready",
    "playback_ids": [
      {
        "id": "xyz789",
        "policy": "public"
      }
    ],
    "duration": 323.5,
    "created_at": "1705296600",
    "updated_at": "1705296700",
    "aspect_ratio": "16:9",
    "video_quality": "basic",
    "resolution_tier": "1080p",
    "encoding_tier": "baseline",
    "max_stored_frame_rate": 30.0,
    "tracks": [
      {
        "type": "video",
        "id": "track_video_001",
        "duration": 323.4,
        "max_width": 1920,
        "max_height": 1080,
        "max_frame_rate": 30.0
      },
      {
        "type": "audio",
        "id": "track_audio_001",
        "duration": 323.5,
        "max_channels": 2,
        "max_channel_layout": "stereo"
      }
    ],
    "static_renditions": {
      "files": [
        {
          "id": "rendition_001",
          "type": "mp4",
          "status": "ready",
          "resolution": "1080p",
          "name": "high.mp4",
          "ext": "mp4"
        }
      ]
    }
  }
}
```

**フィールド:**
- `success` (boolean): 常に`true`
- `command` (string): "show"
- `data` (object): アセット詳細データ（Mux API完全レスポンス）

**終了コード:**
- `0`: 成功
- `1`: 無効なアセットID
- `2`: 未認証
- `3`: API通信エラー

---

### delete - 動画削除

指定したアセットIDの動画を削除します。

**構文:**
```bash
vidyeet delete <asset_id> [--force]
```

**引数:**
- `asset_id`: アセットID（必須）

**フラグ:**
- `--force`: 確認プロンプトをスキップ

**人間向け出力例（stderr、通常）:**
```
⚠️  WARNING: You are about to delete the following asset:
   Asset ID: abc123xyz

This action cannot be undone. All video data will be permanently deleted.

Type 'yes' to confirm deletion: yes

✓ Asset deleted successfully.
```

**人間向け出力例（stderr、--force）:**
```
✓ Asset deleted successfully.
```

**機械向け出力例（stdout、--machine）:**
```json
{
  "success": true,
  "command": "delete",
  "asset_id": "abc123xyz"
}
```

**フィールド:**
- `success` (boolean): 常に`true`
- `command` (string): "delete"
- `asset_id` (string): 削除されたアセットID

**終了コード:**
- `0`: 成功（またはキャンセル）
- `1`: 無効なアセットID
- `2`: 未認証
- `3`: API通信エラー

**注意:** `--machine`フラグ指定時は、`--force`が自動的に有効になります（確認プロンプトなし）。

---

### upload - 動画アップロード

動画ファイルをMuxにアップロードします。

**構文:**
```bash
vidyeet upload <file_path> [--progress]
```

**引数:**
- `file_path`: アップロードする動画ファイルのパス（必須）

**フラグ:**
- `--progress`: 進捗情報をJSONL形式で出力（`--machine`フラグと併用）

**人間向け出力例（stderr）:**
```
Uploading video.mp4...
[████████████████████████████████] 100% (10.0 MB / 10.0 MB)

Waiting for asset creation...

✓ Upload completed successfully!
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Asset ID: abc123xyz

  🎬 HLS Streaming URL (ready now):
     https://stream.mux.com/xyz789.m3u8

  📦 MP4 Download URL:
     Status: ✓ Ready
     https://stream.mux.com/xyz789/highest.mp4
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**機械向け出力例（stdout、--machine）:**
```json
{
  "success": true,
  "command": "upload",
  "asset_id": "abc123xyz",
  "playback_id": "xyz789",
  "hls_url": "https://stream.mux.com/xyz789.m3u8",
  "mp4_url": "https://stream.mux.com/xyz789/highest.mp4",
  "mp4_status": "ready",
  "file_path": "video.mp4",
  "file_size": 10485760,
  "file_format": "mp4",
  "deleted_old_videos": 0
}
```

**フィールド:**
- `success` (boolean): 常に`true`
- `command` (string): "upload"
- `asset_id` (string): 生成されたアセットID
- `playback_id` (string | null): 再生ID
- `hls_url` (string | null): HLS再生URL
- `mp4_url` (string | null): MP4ダウンロードURL
- `mp4_status` (string): "ready"または"generating"
- `file_path` (string): アップロードしたファイルパス
- `file_size` (number): ファイルサイズ（バイト）
- `file_format` (string): ファイル形式
- `deleted_old_videos` (number): 削除された古い動画の数

**進捗通知（--machine --progress）:**

`--machine --progress`を指定すると、JSONL形式（1行1JSON）で進捗が出力されます。

```json
{"phase":"validating_file","file_path":"video.mp4"}
{"phase":"file_validated","file_name":"video.mp4","size_bytes":10485760,"format":"mp4"}
{"phase":"creating_direct_upload","file_name":"video.mp4"}
{"phase":"direct_upload_created","upload_id":"abc123"}
{"phase":"uploading_file","file_name":"video.mp4","size_bytes":10485760,"total_chunks":10}
{"phase":"uploading_chunk","current_chunk":1,"total_chunks":10,"bytes_sent":1048576,"total_bytes":10485760}
{"phase":"file_uploaded","file_name":"video.mp4","size_bytes":10485760}
{"phase":"waiting_for_asset","upload_id":"abc123","elapsed_secs":5}
{"phase":"completed","asset_id":"abc123xyz"}
```

**進捗フェーズ:**
- `validating_file`: ファイル検証中
- `file_validated`: ファイル検証完了
- `creating_direct_upload`: アップロードURL作成中
- `direct_upload_created`: アップロードURL作成完了
- `uploading_file`: アップロード開始
- `uploading_chunk`: チャンクアップロード中
- `file_uploaded`: アップロード完了
- `waiting_for_asset`: アセット作成待機中
- `completed`: 処理完了

**終了コード:**
- `0`: 成功
- `1`: ファイルエラー（不存在、サイズ超過、形式不正）
- `2`: 未認証
- `3`: ネットワークエラー、API通信エラー

---

### help - ヘルプ表示

利用可能なコマンドの一覧とヘルプを表示します。

**構文:**
```bash
vidyeet help
```

**人間向け出力例（stderr）:**
```
vidyeet-cli - Mux Video Upload CLI Tool

USAGE:
    vidyeet [FLAGS] <COMMAND> [ARGS]

FLAGS:
    --machine    Output results in machine-readable JSON format

COMMANDS:
    login        Authenticate with Mux API
    logout       Remove stored credentials
    status       Check authentication status
    list         List all uploaded videos
    show         Show video details
    delete       Delete a video
    upload       Upload a video file
    help         Show this help message

EXAMPLES:
    vidyeet login
    vidyeet upload video.mp4
    vidyeet --machine list | ConvertFrom-Json

For more information, visit: https://github.com/k4zunoko/vidyeet-cli
```

**機械向け出力例（stdout、--machine）:**
```json
{
  "success": true,
  "command": "help"
}
```

**終了コード:**
- `0`: 成功

## エラーレスポンス

### 共通エラーレスポンス形式（--machine）

```json
{
  "success": false,
  "error": {
    "message": "Error description",
    "exit_code": 1,
    "hint": "Helpful suggestion for the user"
  }
}
```

**フィールド:**
- `success` (boolean): 常に`false`
- `error` (object): エラー詳細
  - `message` (string): エラーメッセージ
  - `exit_code` (number): 終了コード（1/2/3）
  - `hint` (string | null): ユーザー向けヒント

### エラー例

**ファイルが見つからない（終了コード: 1）:**
```json
{
  "success": false,
  "error": {
    "message": "File not found: video.mp4",
    "exit_code": 1,
    "hint": "Check that the file path is correct and the file exists."
  }
}
```

**未認証（終了コード: 2）:**
```json
{
  "success": false,
  "error": {
    "message": "Authentication token not found",
    "exit_code": 2,
    "hint": "Please run 'vidyeet login' to authenticate."
  }
}
```

**ネットワークエラー（終了コード: 3）:**
```json
{
  "success": false,
  "error": {
    "message": "Network error: connection timeout",
    "exit_code": 3,
    "hint": null
  }
}
```

## 実用例

### PowerShell: CI/CDパイプラインでの動画アップロード

```powershell
# 環境変数からログイン
$credentials = "$env:MUX_TOKEN_ID`n$env:MUX_TOKEN_SECRET"
$loginResult = $credentials | vidyeet --machine login --stdin | ConvertFrom-Json

if (-not $loginResult.success) {
    Write-Error "Login failed: $($loginResult.error.message)"
    exit $loginResult.error.exit_code
}

# アップロード
$uploadResult = vidyeet --machine upload video.mp4 | ConvertFrom-Json

if ($uploadResult.success) {
    Write-Host "✓ Upload successful!"
    Write-Host "Asset ID: $($uploadResult.asset_id)"
    Write-Host "HLS URL: $($uploadResult.hls_url)"
} else {
    Write-Error "Upload failed: $($uploadResult.error.message)"
    exit $uploadResult.error.exit_code
}
```

### Bash: バッチ処理での複数動画アップロード

```bash
#!/bin/bash

# ログイン
echo -e "$MUX_TOKEN_ID\n$MUX_TOKEN_SECRET" | vidyeet --machine login --stdin
if [ $? -ne 0 ]; then
    echo "Login failed"
    exit 1
fi

# 複数ファイルのアップロード
for file in *.mp4; do
    echo "Uploading $file..."
    RESULT=$(vidyeet --machine upload "$file")
    
    if [ $? -eq 0 ]; then
        ASSET_ID=$(echo $RESULT | jq -r '.asset_id')
        echo "✓ $file uploaded as $ASSET_ID"
    else
        ERROR=$(echo $RESULT | jq -r '.error.message')
        echo "✗ $file failed: $ERROR"
    fi
done
```

### PowerShell: 進捗をリアルタイムでWebhookに送信

```powershell
$lines = vidyeet --machine upload video.mp4 --progress

foreach ($line in $lines) {
    $json = $line | ConvertFrom-Json
    
    if ($json.success -ne $null) {
        # 最終結果
        Invoke-WebRequest -Uri "https://webhook.site/xxx" `
            -Method POST `
            -Body ($json | ConvertTo-Json) `
            -ContentType "application/json"
    } elseif ($json.phase -eq "uploading_chunk") {
        # 進捗をWebhookに送信
        $progress = @{
            phase = $json.phase
            percent = ($json.bytes_sent / $json.total_bytes) * 100
            current_chunk = $json.current_chunk
            total_chunks = $json.total_chunks
        }
        Invoke-WebRequest -Uri "https://webhook.site/xxx" `
            -Method POST `
            -Body ($progress | ConvertTo-Json) `
            -ContentType "application/json"
    }
}
```

## 設計上の注意事項

### 出力の一貫性

- すべてのコマンドが同じJSON構造（`success`, `command`, データフィールド）を返す
- エラーも同じ構造（`success: false`, `error`オブジェクト）

### 標準入出力の使い分け

- **stdin**: 認証情報の入力（`--stdin`）
- **stdout**: 機械可読JSON（`--machine`指定時のみ）
- **stderr**: 人間向けメッセージ（進捗、エラー、結果）

### 進捗通知の設計

- JSONL形式（1行1JSON）で出力
- 最終結果も1つのJSONとして出力
- 各行を個別にパース可能

### APIデータの完全性

- `--machine`フラグ使用時、Mux APIの完全なレスポンスを返す
- 将来のAPI拡張に対応（新しいフィールドが追加されても互換性を保つ）

## バージョン互換性

### 現在のバージョン: 1.0

#### 保証される互換性

- JSON構造の後方互換性（既存フィールドの削除・型変更なし）
- 終了コードの意味（1/2/3の分類）
- コマンド名とフラグ名

#### 将来追加される可能性のあるフィールド

- アセットデータに新しいメタデータフィールド
- 進捗通知に新しいフェーズ
- エラーレスポンスに追加の診断情報

**互換性の原則:** 新しいフィールドの追加はOK、既存フィールドの削除・型変更はNG

## 参考資料

- [Command Line Interface Guidelines](https://clig.dev/)
- [UNIX Philosophy](https://en.wikipedia.org/wiki/Unix_philosophy)
- [JSON Lines (JSONL)](https://jsonlines.org/)
- [Semantic Versioning](https://semver.org/)