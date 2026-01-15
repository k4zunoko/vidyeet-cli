# vidyeet-cli

**Mux Video対応動画アップロードCLIツール**

**言語**: Rust  
**アーキテクチャ**: Clean Architecture（5層構成）  
**目的**: Mux Video APIを利用した動画のアップロード、管理、配信URLの取得

---

## プロジェクト概要

vidyeet-cliは、コマンドラインから動画をMux Videoにアップロードし、HLS/MP4の配信URLを取得するツールです。

### 主要機能

- **認証管理**: Mux Access Tokenによるログイン/ログアウト
- **動画アップロード**: Direct Upload APIによる安全なアップロード
- **動画管理**: 一覧取得、詳細表示、削除
- **機械可読出力**: JSON形式でのスクリプト連携（`--machine`フラグ）
- **進捗通知**: JSONL形式でのリアルタイム進捗表示

### 技術スタック

- **言語**: Rust 2024 edition
- **非同期**: tokio
- **HTTP通信**: reqwest
- **設定管理**: TOML
- **エラーハンドリング**: anyhow + thiserror

---

## 開発コマンド

```powershell
# ビルド
cargo build

# 実行
cargo run -- upload video.mp4

# パイプライン（JSON出力）
cargo run -- --machine list | ConvertFrom-Json

# テスト（シングルスレッド推奨）
cargo test -- --test-threads=1

# Lint & フォーマット
cargo clippy
cargo fmt
```

---

## ドキュメント構成

このプロジェクトのドキュメントは、以下のように整理されています。

### コアドキュメント

| ドキュメント | 内容 |
|-------------|------|
| [DESIGN_PHILOSOPHY.md](docs/DESIGN_PHILOSOPHY.md) | 設計思想、Clean Architecture、責務分離、設計判断の根拠 |
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | レイヤー構造、データフロー、モジュール構成 |
| [ERROR_HANDLING.md](docs/ERROR_HANDLING.md) | エラー型の定義、終了コード、エラーハンドリングフロー |
| [CLI_CONTRACT.md](docs/CLI_CONTRACT.md) | CLIインターフェース仕様、コマンドリファレンス、機械可読API |
| [CONFIGURATION.md](docs/CONFIGURATION.md) | AppConfig（静的設定）とUserConfig（動的設定）の管理 |
| [TESTING_STRATEGY.md](docs/TESTING_STRATEGY.md) | テスト方針、実行方法、カバレッジ目標 |

### 外部API・仕様ドキュメント

| ドキュメント | 内容 |
|-------------|------|
| [API_VIDEO_SPECIFICATION.md](docs/API_VIDEO_SPECIFICATION.md) | Mux Video API仕様、エンドポイント、認証方法 |
| [API_UPLOAD_PROGRESS.md](docs/API_UPLOAD_PROGRESS.md) | アップロード進捗の実装ノート（Direct Upload） |
| [AUTHENTICATION_DESIGN.md](docs/AUTHENTICATION_DESIGN.md) | 認証フロー設計、Access Token管理 |

### ユーザー向けドキュメント

| ドキュメント | 内容 |
|-------------|------|
| [README.md](README.md) | セットアップ、使い方、コマンド例 |
| [MACHINE_API.md](MACHINE_API.md) | 機械可読API詳細リファレンス（`--machine`フラグ） |

---

## アーキテクチャ概要

### レイヤー構造（Clean Architecture）

```
┌─────────────────────────────────────────┐
│  プレゼンテーション層                      │
│  main.rs, cli.rs, presentation/         │
│  - CLI引数解析、出力フォーマット          │
└────────────┬────────────────────────────┘
             ↓ 依存（外→内）
┌─────────────────────────────────────────┐
│  アプリケーション層                        │
│  commands/                               │
│  - login, logout, status, list,          │
│    show, delete, upload, help            │
└──────────┬────────────┬─────────────────┘
           ↓            ↓
    ┌───────────┐ ┌──────────┐ ┌──────────┐
    │ドメイン層  │ │  設定層   │ │インフラ層 │
    │domain/    │ │config/   │ │api/      │
    │バリデー   │ │静的/動的  │ │HTTP通信  │
    │ション     │ │設定管理   │ │Mux API   │
    └───────────┘ └──────────┘ └──────────┘
```

詳細は [`ARCHITECTURE.md`](docs/ARCHITECTURE.md) を参照。

### 設計原則

1. **Clean Architectureによる責務分離**
   - 外側から内側への依存のみ許可
   - ドメイン層はインフラや表示を知らない

2. **マジックナンバーの完全排除**
   - すべての定数を`config/app.rs`に集約
   - コンパイル時評価による実行時コストゼロ

3. **エラーの階層化**
   - DomainError（ビジネスルール違反）
   - ConfigError（設定問題）
   - InfraError（外部通信エラー）

4. **CLI出力の明示的制御**
   - `--machine`フラグで機械可読JSON出力
   - stdout/stderrの明確な分離

詳細は [`DESIGN_PHILOSOPHY.md`](docs/DESIGN_PHILOSOPHY.md) を参照。

---

## コマンド一覧

| コマンド | 機能 |
|---------|------|
| `login` | 認証情報入力・保存（HTTP Basic認証テスト実行） |
| `logout` | 認証情報削除 |
| `status` | 認証状態確認（APIアクセステスト実行） |
| `list` | アセット一覧取得 |
| `show <asset_id>` | アセット詳細情報取得 |
| `delete <asset_id>` | アセット削除（確認プロンプト付き、`--force`でスキップ可） |
| `upload <file>` | ファイル検証 → Direct Upload作成 → アップロード → Asset取得 |
| `help` | ヘルプ表示 |

詳細は [`CLI_CONTRACT.md`](docs/CLI_CONTRACT.md) を参照。

---

## 終了コード

| コード | 種別 | 例 |
|-------|------|---|
| 0 | 成功 | - |
| 1 | ユーザーエラー | ファイル不正、形式無効 |
| 2 | 設定エラー | トークン無効、設定破損 |
| 3 | システムエラー | I/O障害、ネットワーク障害 |

詳細は [`ERROR_HANDLING.md`](docs/ERROR_HANDLING.md) を参照。

---

## ドキュメント管理方針

**重要**: これらのドキュメントは実装状況と常に一致するよう、以下のタイミングで更新してください:

1. **設計判断の変更時**: DESIGN_PHILOSOPHY.md, 該当レイヤのドキュメント
2. **実装追加時**: 該当レイヤのドキュメント
3. **テスト追加時**: TESTING_STRATEGY.md
4. **エラーハンドリング変更時**: ERROR_HANDLING.md

GitHub Copilotは常にこれらのドキュメントを参照してサポートを提供します。

---

## LLM向けメタ情報

### このドキュメント群について

**AGENTS.mdとdocs/配下のドキュメント群は、GitHub CopilotなどのLLM言語モデルがプロジェクト状況を正確に理解するために設計されています。**

### ドキュメント設計の責務

#### AGENTS.md
- **プロジェクトの最低限の概要**（目的、技術スタック、アーキテクチャ）
- **docs/配下のドキュメントへのナビゲーション**（索引・使用ガイド）
- **このメタ情報**（LLMがドキュメント管理を理解するため）

#### docs/配下のドキュメント
- **詳細な設計方針と実装指針**（AGENTS.mdには書かない）
- **設計判断の根拠**（なぜこの設計にしたのか）
- **具体的な実装例とコードスニペット**
- **変更許容性のガイドライン**（何を変更してよいか、何を保護すべきか）
- **レイヤごとの責務と境界**

### ドキュメント品質の原則

1. **具体性**: 抽象的な記述ではなく、コード例や具体的な数値を含める
2. **根拠**: 設計判断には必ず理由を明記（パフォーマンス、保守性、安全性など）
3. **最新性**: 実装と常に一致させる（古い情報は削除または更新）
4. **ナビゲーション**: AGENTS.mdから各ドキュメントへの明確な案内
5. **文脈**: LLMが次回セッションで読んでも理解できる十分な文脈情報

### 注意事項

- **AGENTS.mdに詳細を書かない**: 詳細はdocs/配下に分離
- **重複を避ける**: 同じ情報は1箇所のみに記載し、相互参照を使用
- **実装との一致**: ドキュメントと実装が乖離した場合、必ず同期する

---

## 参考資料

### 外部リソース

- [Mux Video API Reference](https://docs.mux.com/api-reference/video/assets)
- [Mux Direct Uploads ガイド](https://docs.mux.com/guides/upload-files-directly)
- [Rust公式ドキュメント](https://doc.rust-lang.org/)
- [Clean Architecture](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)

---

**Author**: [@k4zunoko](https://github.com/k4zunoko)  
**License**: MIT  
**Built with**: ❤️ and Rust 🦀