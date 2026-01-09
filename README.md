# vidyeet-cli

**Mux Video対応動画アップロードCLIツール**

[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org)

## 概要

`vidyeet-cli`は、[Mux Video](https://www.mux.com/)のAPIを利用して動画をアップロードするためのコマンドラインツールです。

---

## インストール

### 前提条件

- Rust 2024 edition以降（`rustc 1.75+`推奨）
- Mux アカウント（[https://mux.com/](https://mux.com/)）

### ビルド

```powershell
git clone https://github.com/k4zunoko/vidyeet-cli.git
cd vidyeet-cli

cargo build --release
```

---

## 使い方

### 1. ログイン

Muxダッシュボードで取得したAccess Token IDとSecretを使って認証します。

#### 対話形式ログイン（推奨）

```powershell
vidyeet login
```

対話形式で認証情報を入力します：

```
Logging in to Mux Video...

Please enter your Mux Access Token credentials.
You can find them at: https://dashboard.mux.com/settings/access-tokens

Access Token ID: abc123xyz
Access Token Secret: ****

Login successful.
Authentication credentials have been saved.
```

#### 標準入力からのログイン（CI/CD向け）

機械的な処理やスクリプトから認証情報を供給する場合は `--stdin` オプションを使用します：

```powershell
# 環境変数から認証情報を供給
echo "$env:MUX_TOKEN_ID`n$env:MUX_TOKEN_SECRET" | vidyeet login --stdin

# ファイルから認証情報を読み込み
Get-Content credentials.txt | vidyeet login --stdin
```

**credentials.txt の形式:**
```
your-token-id
your-token-secret
```

**セキュリティ上の注意:**
- `--stdin` を使用することで、認証情報がシェル履歴に記録されることを防げます
- 認証情報は環境変数やファイルから読み込むことを推奨します

**Access Tokenの取得方法:**
1. [Mux Dashboard](https://dashboard.mux.com/)にログイン
2. **Settings → Access Tokens** へ移動
3. **Generate new token** をクリック
4. Token IDとSecretをコピー

### 2. 動画をアップロード

```powershell
vidyeet upload video.mp4
```

**出力例（通常モード）:**

```
✓ Upload completed successfully!
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Asset ID: asset_abc123xyz

  🎬 HLS Streaming URL (ready now):
     https://stream.mux.com/xyz123.m3u8

  📦 MP4 Download URL:
     Status: ✓ Ready
     https://stream.mux.com/xyz123/highest.mp4
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**出力例（--machineフラグ指定時）:**

```powershell
vidyeet --machine upload video.mp4 | ConvertFrom-Json
```

```json
{
  "success": true,
  "command": "upload",
  "asset_id": "asset_abc123xyz",
  "playback_id": "xyz123",
  "hls_url": "https://stream.mux.com/xyz123.m3u8",
  "mp4_url": "https://stream.mux.com/xyz123/highest.mp4",
  "mp4_status": "ready",
  "file_path": "video.mp4",
  "file_size": 10485760,
  "file_format": "mp4",
  "deleted_old_videos": 0
}
```

### 3. 動画リストを取得

アップロード済みの動画一覧を表示します。

```powershell
vidyeet list
```

### 4. 動画の詳細を表示

指定したアセットIDの詳細情報を表示します。

```powershell
vidyeet show <asset_id>
```

**表示される情報:**

```
Asset Details:
==============
Asset ID:       lJ4bGGsp7ZlPf02nMg015W02iHQLN9XnuuLRBsPS00xqd68
Status:         ready
Duration:       0:24 (23.86s)
Aspect Ratio:   16:9
Video Quality:  basic
Created At:     2021-01-05 12:46:08 +09:00

Playback Information:
--------------------
Playback ID #1: vAFLI2eKFFicXX00iHBS2vqt5JjJGg5HV6fQ4Xijgt1I
  Policy:       public
HLS URL:        https://stream.mux.com/vAFLI2eKFFicXX00iHBS2vqt5JjJGg5HV6fQ4Xijgt1I.m3u8
MP4 URL:        https://stream.mux.com/vAFLI2eKFFicXX00iHBS2vqt5JjJGg5HV6fQ4Xijgt1I/high.mp4

Tracks:
-------
Track #1: video (duration: 23.82s)
Track #2: audio (duration: 23.82s)
```

### 5. 動画を削除

指定したアセットIDの動画を削除します。

```powershell
vidyeet delete <asset_id>
```

**確認プロンプト:**

```
⚠️  WARNING: You are about to delete the following asset:
   Asset ID: asset_abc123xyz

This action cannot be undone. All video data will be permanently deleted.

Type 'yes' to confirm deletion: 
```

**強制削除（確認をスキップ）:**

```powershell
vidyeet delete <asset_id> --force
```

### 6. ログアウト

認証情報を削除します。

```powershell
vidyeet logout
```

### 7. ステータス確認

認証状態を確認します。

```powershell
vidyeet status
```

### 8. 機械可読出力（スクリプト向け）

`--machine`フラグを使用すると、JSON形式で結果を出力します。すべてのコマンドで成功時・失敗時ともにJSON形式で出力されます。

#### 成功時の出力例

```powershell
vidyeet --machine status
```

```json
{
  "command": "status",
  "is_authenticated": true,
  "success": true,
  "token_id": "abcd***1234"
}
```

#### 失敗時の出力例

```powershell
vidyeet --machine list  # 未認証状態
```

```json
{
  "success": false,
  "error": {
    "message": "List command failed",
    "exit_code": 2,
    "hint": "Please run 'vidyeet login' to authenticate with api.video."
  }
}
```

#### 対応コマンド

```powershell
vidyeet --machine login --stdin    # 標準入力からログイン
vidyeet --machine status           # ステータス確認
vidyeet --machine list             # 動画一覧
vidyeet --machine show <asset_id>  # 動画詳細
vidyeet --machine upload video.mp4 # アップロード
vidyeet --machine delete <asset_id> --force  # 削除
```

**注意**: 
- `--machine`はグローバルフラグのため、コマンド名の前に指定します
- エラー発生時も必ずJSON形式で出力されるため、スクリプトでのパースが容易です
- 終了コード（exit_code）は標準エラー処理に従います（0: 成功, 1: ユーザーエラー, 2: 設定エラー, 3: システムエラー）

### 9. ヘルプ

```powershell
vidyeet help
```
---

### 機械可読出力の仕様

`--machine`フラグを使用すると、JSON形式で構造化されたデータを標準出力に出力します。

**出力形式の違い:**

- **通常実行**: 人間が読みやすい簡略版のJSON（`videos`/`playback_ids`等）
- **--machineフラグ**: Mux APIの完全なレスポンスデータを含む（`raw_assets`/`raw_asset`フィールド）

**対応コマンド:**

| コマンド | 通常出力 | --machine出力 |
|---------|---------|--------------|
| `list` | `videos`配列（簡略版） | `raw_assets`配列（完全なAssetData） |
| `show` | 基本フィールドのみ | `raw_asset`オブジェクト（完全なAssetData） |
| `upload` | 成功メッセージとURL | 同左（変更なし） |
| `delete` | asset_id | 同左（変更なし） |

**raw_assets/raw_assetに含まれる追加フィールド例:**

- `resolution_tier`: 解像度ティア（1080p, 720pなど）
- `encoding_tier`: エンコーディングティア（baseline, smartなど）
- `max_stored_resolution`: 最大保存解像度
- `max_stored_frame_rate`: 最大保存フレームレート
- `tracks[].id`: トラックID
- `tracks[].max_width`: 最大幅
- `tracks[].max_height`: 最大高さ
- その他、Mux API公式仕様の全フィールド

**使用例:**

```bash
# 通常の人間向け出力
vidyeet list

# スクリプト向けの完全なAPIデータ取得
vidyeet --machine list | jq '.raw_assets[0].resolution_tier'
vidyeet --machine show <asset_id> | jq '.raw_asset.encoding_tier'
```
---

## 作者

[@k4zunoko](https://github.com/k4zunoko)

---

**Built with ❤️ and Rust 🦀**
