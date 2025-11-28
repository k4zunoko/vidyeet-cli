# vidyeet-cli

**Mux Video対応動画アップロードCLIツール（Rust学習プロジェクト）**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org)

## 概要

`vidyeet-cli`は、[Mux Video](https://www.mux.com/)のAPIを利用して動画をアップロードするためのコマンドラインツールです。このプロジェクトは作者の**Rustの学習過程**で作成されたもので、実践的にアーキテクチャパターンやRustの言語機能を取り入れています。

### 主な機能

- 🎬 Mux Videoへの動画アップロード（Direct Upload API使用）
- 📦 HLS/MP4形式での動画配信URL取得
- 🖥️ UNIX哲学に基づくstdout/stderr分離
- 📝 JSON出力対応（スクリプトから利用可能）

## 目次

- [インストール](#インストール)
- [使い方](#使い方)
- [設計方針](#設計方針)
- [アーキテクチャ](#アーキテクチャ)
- [エラーハンドリング](#エラーハンドリング)
- [開発](#開発)
- [技術スタック](#技術スタック)
- [ライセンス](#ライセンス)

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

```powershell
vidyeet login
```

対話形式で認証情報を入力します：

```
Enter your Mux Access Token ID: abc123xyz
Enter your Mux Access Token Secret: ****
Authenticating...
✓ Login successful!
```

**Access Tokenの取得方法:**
1. [Mux Dashboard](https://dashboard.mux.com/)にログイン
2. **Settings → Access Tokens** へ移動
3. **Generate new token** をクリック
4. Token IDとSecretをコピー

### 2. 動画をアップロード

```powershell
vidyeet upload video.mp4
```

**出力例（TTY接続時）:**

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

**出力例（パイプライン）:**

```powershell
vidyeet upload video.mp4 | ConvertFrom-Json
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

### 3. ログアウト

認証情報を削除します。

```powershell
vidyeet logout
```

### 4. ヘルプ

```powershell
vidyeet help
```

---

## 設計方針

このプロジェクトでは、以下の設計原則を採用しています。

### Clean Architecture（レイヤー構造）

4層構成で依存方向を厳密に管理しています。

```
┌─────────────────────────────────────────┐
│  プレゼンテーション層                     │
│  main.rs, cli.rs, error_severity.rs     │
│  (ユーザーI/O、終了コード決定)            │
└────────────┬────────────────────────────┘
             ↓
┌─────────────────────────────────────────┐
│  アプリケーション層                       │
│  commands/ (login, logout, upload)      │
│  (ユースケース実装、anyhowでエラー集約)    │
└────────────┬────────────────────────────┘
             ↓
┌──────────────┬──────────────┬───────────┐
│ ドメイン層    │ 設定層        │インフラ層 │
│ domain/      │ config/      │ api/      │
│ (ビジネス     │ (静的/動的    │(HTTP通信、│
│  ルール)      │ 設定)        │ 認証)     │
└──────────────┴──────────────┴───────────┘
```

**依存方向の原則:**
- 外側から内側のみ依存（内側は外側を知らない）
- `ErrorSeverity`は全層から使用される独立モジュール

### エラーハンドリング戦略

#### 終了コード

| コード | 種別 | 例 |
|-------|------|---|
| 0 | 成功 | - |
| 1 | ユーザー入力エラー | ファイル不正、形式無効 |
| 2 | 設定エラー | トークン無効、設定破損 |
| 3 | システムエラー | I/O障害、ネットワーク障害 |

#### エラー型

各層で専用のエラー型を定義し、`severity()`と`hint()`メソッドで統一的に処理します。

- **DomainError**: ビジネスルール違反（ファイルバリデーション）
- **ConfigError**: 設定ファイルの問題（認証情報、TOML解析）
- **InfraError**: 外部通信エラー（HTTP、ネットワーク）

---

## アーキテクチャ

### ディレクトリ構造

```
vidyeet-cli/
├── src/
│   ├── main.rs                  # エントリーポイント、エラーハンドリング
│   ├── cli.rs                   # CLI引数解析、出力制御
│   ├── error_severity.rs        # 終了コード決定ロジック（独立）
│   │
│   ├── commands/                # アプリケーション層
│   │   ├── mod.rs
│   │   ├── login.rs             # ログインコマンド
│   │   ├── logout.rs            # ログアウトコマンド
│   │   ├── upload.rs            # アップロードコマンド
│   │   ├── help.rs              # ヘルプコマンド
│   │   └── result.rs            # コマンド結果型
│   │
│   ├── domain/                  # ドメイン層
│   │   ├── mod.rs
│   │   ├── validator.rs         # ファイルバリデーション
│   │   └── error.rs             # DomainError定義
│   │
│   ├── config/                  # 設定層
│   │   ├── mod.rs
│   │   ├── app.rs               # 静的設定（コンパイル時定数）
│   │   ├── user.rs              # 動的設定（TOML）
│   │   └── error.rs             # ConfigError定義
│   │
│   └── api/                     # インフラ層
│       ├── mod.rs
│       ├── client.rs            # HTTPクライアント
│       ├── auth.rs              # HTTP Basic認証
│       ├── types.rs             # API型定義
│       └── error.rs             # InfraError定義
│
├── Cargo.toml
└── README.md
```

### モジュール間の依存関係

```
main.rs → cli.rs → commands/* → {domain, config, api}
                  ↓
           error_severity.rs (全層から参照可能)
```

---

## 技術スタック

### 依存クレート

| クレート | バージョン | 用途 |
|---------|-----------|------|
| [anyhow](https://docs.rs/anyhow/) | 1.0 | アプリケーション層エラー集約 |
| [thiserror](https://docs.rs/thiserror/) | 1.0 | 構造化エラー定義（derive macro） |
| [serde](https://docs.rs/serde/) | 1.0 | JSON/TOMLシリアライゼーション |
| [serde_json](https://docs.rs/serde_json/) | 1.0 | JSON処理 |
| [toml](https://docs.rs/toml/) | 0.8 | TOML設定ファイル |
| [dirs](https://docs.rs/dirs/) | 5.0 | プラットフォーム固有パス取得 |
| [reqwest](https://docs.rs/reqwest/) | 0.11 | HTTP通信（非同期） |
| [tokio](https://docs.rs/tokio/) | 1.0 | 非同期ランタイム |
| [base64](https://docs.rs/base64/) | 0.21 | HTTP Basic認証ヘッダー生成 |

### アップロード処理フロー

```
1. ファイルバリデーション
   ↓
2. 認証情報読込（config.toml）
   ↓
3. Direct Upload URL作成（POST /video/v1/uploads）
   ↓ [容量制限エラー時]
   ├→ 最古アセット削除
   └→ 再試行
   ↓
4. ファイルアップロード（PUT）
   ↓
5. アセット作成完了をポーリング（2秒間隔、最大300秒）
   ↓
6. HLS/MP4 URL取得
```

### Mux API仕様

詳細は [Mux API Reference](https://www.mux.com/docs/api-reference/video/assets) を参照。

**主要エンドポイント:**

- `POST /video/v1/uploads` - Direct Upload作成
- `GET /video/v1/uploads/{UPLOAD_ID}` - Upload状態確認
- `GET /video/v1/assets/{ASSET_ID}` - Asset情報取得
- `GET /video/v1/assets` - Asset一覧取得
- `DELETE /video/v1/assets/{ASSET_ID}` - Asset削除

**認証:** HTTP Basic認証（Access Token ID/Secret）

---

## 使用している主なRust機能

このプロジェクトで使用しているRustの機能：

- **所有権とライフタイム**: 参照と借用を活用した安全なメモリ管理
- **エラーハンドリング**: `thiserror`と`anyhow`を組み合わせた階層的なエラー処理
- **非同期プログラミング**: `tokio`ランタイムによるasync/await
- **トレイトシステム**: 共通インターフェース（`HasSeverity`）による抽象化
- **モジュールシステム**: Clean Architectureに基づいた明確な責務分離
- **マクロ**: deriveマクロ（`Serialize`, `Deserialize`, `Error`）の活用
- **型システム**: `Result`/`Option`による安全なエラー処理、コンパイル時定数

---

## ライセンス

このプロジェクトはMITライセンスの下で公開されています。

---

## 参考資料

- [Mux Video API ドキュメント](https://docs.mux.com/)
- [Mux Direct Uploads ガイド](https://docs.mux.com/guides/upload-files-directly)
- [Rust公式ドキュメント](https://doc.rust-lang.org/)
- [Clean Architecture](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)

---

## 作者

[@k4zunoko](https://github.com/k4zunoko)

---

**Built with ❤️ and Rust 🦀**
