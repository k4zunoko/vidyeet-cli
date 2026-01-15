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

Muxダッシュボードで取得したAccess Token IDとSecretを使って認証をおこないます。

#### 対話形式ログイン

```powershell
vidyeet login
```

#### 標準入力からのログイン（CI/CD向け）

機械的な処理やスクリプトから認証情報を供給する場合は `--stdin` オプションを使用します：

```powershell
# 環境変数から認証情報を供給
echo "$env:MUX_TOKEN_ID`n$env:MUX_TOKEN_SECRET" | vidyeet login --stdin

# ファイルから認証情報を読み込み
Get-Content credentials.txt | vidyeet login --stdin
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

### 5. 動画を削除

指定したアセットIDの動画を削除します。
forceフラグを付けると、確認プロンプトなしで削除を実行します。

```powershell
vidyeet delete <asset_id>　[<--force>]
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

### 機械可読出力

すべてのコマンドに `--machine` フラグを付けると、JSON形式で構造化されたデータを出力します。
**詳細な仕様とコマンドリファレンスは [`MACHINE_API.md`](MACHINE_API.md) を参照してください。**

```powershell
vidyeet --machine status
vidyeet --machine upload video.mp4
vidyeet --machine list
```

---

## 作者

[@k4zunoko](https://github.com/k4zunoko)

---

**Built with ❤️ and Rust 🦀**
