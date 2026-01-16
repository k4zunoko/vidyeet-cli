# Mux Upload Progress について

## 概要
- Direct Upload（署名付きURLへクライアントからPUT）
  - ネットワークアップロード中の進捗はAPIからは出ないため、自前で送信バイトをカウントして%を出します。URLは再開可能なので、失敗時は最後に成功したオフセットから再送すればOKです。
  - MuxのWeb用コンポーネント Mux Uploader は内部でUpChunkを使い、チャンク毎にContent-Rangeを付けてPUTし、progressイベントを発火しています。CLIでも同じ方式を再現可能です。
  

## 進捗イベントの設計

### `uploading_file` フェーズ（v1.1以降）

アップロード開始時に以下の情報を提供：
- `file_name`: ファイル名
- `size_bytes`: ファイルサイズ
- `total_chunks`: 総チャンク数（v1.1で追加）

**GUI実装での活用例:**
```typescript
case 'uploading_file':
  // プログレスバーの準備
  const progressBar = createProgressBar({
    min: 0,
    max: event.total_chunks,
    current: 0
  });
  // 表示: "Uploading: video.mp4 (100 MB, 5 chunks)"
  setStatus(`Uploading: ${event.file_name} (${formatSize(event.size_bytes)}, ${event.total_chunks} chunks)`);
  break;
```

この設計により、最初の `uploading_chunk` イベントを待たずにUIを準備できます。

### `uploading_chunk` フェーズ

各チャンク送信完了後に以下の情報を提供：
- `current_chunk`: 現在のチャンク番号（1-indexed）
- `total_chunks`: 総チャンク数
- `bytes_sent`: 送信済みバイト数
- `total_bytes`: 総バイト数

**注意**: `current_chunk = 1` が最初のチャンク送信完了を示します。極小ファイルでも必ず最低1回は出力されます。

## CLIでの実装方針（Direct Upload）

Muxの署名付きアップロードURLを取得したら、ローカルファイルをチャンク分割し、各チャンクを個別のPUTで送ります。その際、以下を満たすと堅牢です：

1. チャンク分割

- 例：32MBなど。256KiBの倍数にしておくと、Muxが推奨するUpChunk実装と整合します。 [npmjs.com], [github.com]


2. ヘッダ

- Content-Type: 入力ファイルに合わせる
- Content-Length: 送るチャンクサイズ
- Content-Range: bytes {start}-{end}/{total} を必ずセット
  - 例：総サイズ=100,000,000B、最初の32MBチャンクなら bytes 0-33554431/100000000
- UpChunkがPUT＋Rangeヘッダでチャンク再送・再開を成立させているのと同じ方式です。 [npmjs.com], [github.com]


3. 進捗計算

- 自分で累積送信バイト数（end+1）を足し上げて、sent / total * 100 を表示。
- チャンクが成功（HTTP 200/201/204）したタイミングで更新すれば、素直に動きます。


4. レジューム（途中失敗／再試行）

- 直近の成功チャンクの末尾オフセットをローカルに記録し、次回はそのオフセットから再送。
- ※tusプロトコルのようにHEADでUpload-Offsetを問い合わせる方法も一般論としてありますが、MuxのDirect Uploadはtusサーバではなく、Range付きPUTを前提にした実装（UpChunk方式）なので、HEADでオフセット取得する前提は置かない方が安全です。


## 実装時の重要ポイント

- Content-Range と Content-Length を正しく設定すること。 [npmjs.com], [github.com]
- fetch（Node）では送信中の細粒度イベントは取りづらいので、**チャンク完了ごとに進捗更新する設計が現実的**。より細かい表示をしたいなら、チャンクを小さくして更新頻度を上げるか、低レベルのソケット層で書き込みバイトを監視します。
- 失敗時は指数バックオフで再試行、一定回数超過で中断。UpChunkも再試行を備えています（CLIなら自前実装）。 [npmjs.com]
- **進捗イベントのタイミング**: `uploading_file` でアップロード開始を通知し、各チャンク送信完了後に `uploading_chunk` を通知します。これにより、GUI側で0%のプログレスバーを準備してから実際の進捗を表示できます。


## ありがちな落とし穴／限界

- **tusプロトコルのオフセット問い合わせ（HEADでUpload-Offset）**は一般論として有効ですが、MuxのDirect Uploadはtusのサーバではないため、そのまま使えるとは限りません。Mux公式の実装（UpChunk）にならい、Range付きPUT＋ローカルでのオフセット管理を前提に設計してください。 [tus.io], [npmjs.com]
- URLインジェストではネットワーク転送進捗は不可。**Assetのprogress**のみが指標です。 [mux.com]
- チャンクサイズは大きすぎるとUI更新が疎になる／失敗時の再送コストが高くなる。256KiBの倍数を守りつつ、回線とUXを見て調整。