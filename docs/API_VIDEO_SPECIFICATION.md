# Mux Video API仕様書

## 概要

Mux Videoは動画のアップロード、エンコード、配信を行うための開発者向けビデオプラットフォームです。
アダプティブビットレートストリーミング(HLS)による高品質な動画配信を提供します。

## APIエンドポイント

- **ベースURL:** `https://api.mux.com`
- すべてのリクエストはHTTPS経由で行う必要があります

## 認証

### HTTP Basic認証
Mux APIはHTTP Basic認証を使用します。

- **ユーザー名:** Access Token ID
- **パスワード:** Access Token Secret

認証情報はMuxダッシュボードの[Settings → Access Tokens](https://dashboard.mux.com/settings/access-tokens)で作成・管理できます。

**重要な注意事項:**
- Muxはシークレットキーのハッシュのみを保存し、平文は保存しません
- シークレットキーを紛失した場合は復元できないため、新しいAccess Tokenを作成する必要があります
- シークレットキーが漏洩した場合は、すぐにそのAccess Tokenを無効化してください

**認証例(cURL):**
```bash
curl https://api.mux.com/video/v1/assets \
  -u ${MUX_TOKEN_ID}:${MUX_TOKEN_SECRET}
```


## 主要なエンドポイント

### Direct Uploads API

#### 1. Direct Uploadの作成
- **エンドポイント:** `POST /video/v1/uploads`
- **説明:** 動画を直接アップロードするための署名付きURLを作成

**リクエストボディ:**
```json
{
  "cors_origin": "*",
  "new_asset_settings": {
    "playback_policy": ["public"],
    "video_quality": "basic",
    "meta": {
      "title": "My Video Title",
      "creator_id": "user_123",
      "external_id": "video_456"
    }
  },
  "timeout": 3600
}
```

**パラメータ説明:**
- `cors_origin` (必須): ブラウザから使用する場合のCORSオリジン
- `new_asset_settings` (任意): 作成するアセットの設定
  - `playback_policy`: `["public"]` または `["signed"]`
  - `video_quality`: `"basic"`, `"plus"`, `"premium"` (デフォルト: `"basic"`)
  - `meta`: メタデータオブジェクト
    - `title`: タイトル (最大512文字)
    - `creator_id`: 作成者ID (最大128文字)
    - `external_id`: 外部参照ID (最大128文字)
- `timeout` (任意): 署名付きURLの有効期限(秒) (デフォルト: 3600, 最小: 60, 最大: 604800)
- `test` (任意): テストアップロードかどうか

**レスポンス (201 Created):**
```json
{
  "data": {
    "id": "upload_abc123",
    "timeout": 3600,
    "status": "waiting",
    "new_asset_settings": {
      "playback_policy": ["public"],
      "video_quality": "basic"
    },
    "asset_id": null,
    "error": null,
    "cors_origin": "*",
    "url": "https://storage.googleapis.com/video-storage-us-east1/...",
    "test": false
  }
}
```

**ステータスの種類:**
- `waiting`: アップロード待機中
- `asset_created`: アセット作成完了
- `errored`: エラー発生
- `cancelled`: キャンセル済み
- `timed_out`: タイムアウト

#### 2. 動画ファイルのアップロード
- **メソッド:** `PUT`
- **URL:** Direct Upload作成時に取得した`url`を使用
- **説明:** 取得したURLに動画ファイルをPUTリクエストでアップロード

**アップロード例(cURL):**
```bash
curl -v -X PUT -T video.mp4 "$UPLOAD_URL"
```

### Assets API

#### 3. アセット一覧の取得
- **エンドポイント:** `GET /video/v1/assets`
- **説明:** アカウント内のすべてのアセットを取得

**クエリパラメータ:**
- `limit`: 1ページあたりの件数 (デフォルト: 25)
- `page`: ページ番号 (デフォルト: 1)
- `cursor`: カーソルベースのページネーション用
- `live_stream_id`: 特定のライブストリームのアセットに絞り込み
- `upload_id`: 特定のDirect Uploadから作成されたアセットに絞り込み

**レスポンス例:**
```json
{
  "data": [
    {
      "id": "asset_abc123",
      "status": "ready",
      "duration": 734.25,
      "created_at": "1609869152",
      "aspect_ratio": "16:9",
      "playback_ids": [
        {
          "id": "playback_xyz789",
          "policy": "public"
        }
      ],
      "tracks": [
        {
          "type": "video",
          "max_width": 1920,
          "max_height": 1080,
          "max_frame_rate": 30,
          "duration": 734.166667
        },
        {
          "type": "audio",
          "max_channels": 2,
          "duration": 734.143991
        }
      ],
      "max_stored_resolution": "HD",
      "resolution_tier": "1080p",
      "video_quality": "basic",
      "encoding_tier": "baseline"
    }
  ],
  "next_cursor": "eyJwYWdlX2xpbWl0IjoyNX0..."
}
```

**アセットのステータス:**
- `preparing`: エンコード準備中
- `ready`: 再生可能
- `errored`: エラー発生

#### 4. アセット情報の取得
- **エンドポイント:** `GET /video/v1/assets/{ASSET_ID}`
- **説明:** 特定のアセットの詳細情報を取得

#### 5. アセットの削除
- **エンドポイント:** `DELETE /video/v1/assets/{ASSET_ID}`
- **説明:** アセットとすべての関連データを削除

**リクエスト例:**
```bash
curl https://api.mux.com/video/v1/assets/${ASSET_ID} \
  -X DELETE \
  -H "Content-Type: application/json" \
  -u ${MUX_TOKEN_ID}:${MUX_TOKEN_SECRET}
```

**レスポンス:** 204 No Content (成功時)

#### 6. Direct Upload情報の取得
- **エンドポイント:** `GET /video/v1/uploads/{UPLOAD_ID}`
- **説明:** Direct Uploadのステータスと関連アセットIDを取得

**レスポンス例:**
```json
{
  "data": {
    "id": "upload_abc123",
    "timeout": 3600,
    "status": "asset_created",
    "asset_id": "asset_xyz789",
    "url": "https://storage.googleapis.com/...",
    "cors_origin": "*"
  }
}
```

#### 7. Direct Uploadのキャンセル
- **エンドポイント:** `PUT /video/v1/uploads/{UPLOAD_ID}/cancel`
- **説明:** 進行中のDirect Uploadをキャンセル

## Video Quality レベル

Muxは3つのビデオ品質レベルを提供します:

| Quality Level | 説明 | 用途 |
|--------------|------|------|
| `basic` | 標準品質 (デフォルト) | 一般的な用途 |
| `plus` | 高品質 | より高い品質が必要な場合 |
| `premium` | 最高品質 | プロフェッショナルな用途 |

## レート制限とクォータ

### 無料プランの制限
- **動画アップロード数:** 最大10個
- **ストリーミング:** 月間20時間まで
- **ストレージ:** 100GBまで

### 推奨事項
- アップロード失敗時は古いアセットを自動削除してキュー管理を行う
- `created_at`でソートして最も古いアセットを特定

## アップロードフロー

### 推奨フロー (Direct Upload)

1. **Direct Upload URLを作成** (`POST /video/v1/uploads`)
   - メタデータ(タイトルなど)を設定
   - 署名付きアップロードURLを取得

2. **動画ファイルをアップロード** (PUTリクエスト)
   - 取得したURLに動画ファイルをPUT
   - クライアント側から直接アップロード可能

3. **Upload情報を確認** (`GET /video/v1/uploads/{UPLOAD_ID}`)
   - ステータスが`asset_created`になれば完了
   - `asset_id`を取得

4. **Asset情報取得** (`GET /video/v1/assets/{ASSET_ID}`)
   - `playback_ids`から再生URLを構築
   - ステータスが`ready`になれば再生可能

### Playback URL
再生URLは以下の形式で構築:
```
https://stream.mux.com/{PLAYBACK_ID}.m3u8
```

## エラーレスポンス

一般的なHTTPステータスコード:
- `200` - 成功
- `201` - 作成成功
- `204` - 削除成功(レスポンスボディなし)
- `400` - 不正なリクエスト
- `401` - 認証失敗
- `404` - リソースが見つからない
- `429` - レート制限超過

## その他の機能

### メタデータ
- `meta`オブジェクトで任意のメタデータを設定可能
- `title`: タイトル (最大512文字)
- `creator_id`: 作成者ID (最大128文字)  
- `external_id`: 外部システムでの参照ID (最大128文字)

### Playback Policy
- `public`: 誰でもアクセス可能
- `signed`: 署名付きURLが必要(JWT使用)

### テストモード
- `test: true`でテストアセットを作成
- 課金対象外
- 本番環境での動作確認に使用

## 公式SDK

以下の言語で公式SDKが提供されています:
- Node.js
- Python
- PHP
- Ruby
- Elixir
- Java
- C#

**注意:** Rust向けの公式SDKは現在提供されていないため、HTTP APIを直接使用します。

## セキュリティベストプラクティス

1. **Access Tokenをコードに直接埋め込まない**
   - 環境変数や設定ファイルで管理
   - Gitリポジトリには含めない

2. **Access Tokenの適切な管理**
   - 定期的なローテーション
   - 最小権限の原則に従う
   - 漏洩時はすぐに無効化

3. **HTTPS通信の徹底**
   - すべてのAPI通信はHTTPS経由

4. **Signed Playback Policyの活用**
   - 機密性の高いコンテンツにはJWT署名付きURLを使用

## リファレンス

- **公式ドキュメント:** https://docs.mux.com/
- **API リファレンス:** https://docs.mux.com/api-reference
- **ダッシュボード:** https://dashboard.mux.com/
- **GitHubリポジトリ:** https://github.com/muxinc/

2. **環境変数または外部ファイルで管理**
3. **不要なAPI Keyは削除**
4. **システムごとに異なるAPI Keyを使用**
5. **公開前にコードレビューで漏洩チェック**
