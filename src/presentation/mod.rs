/// プレゼンテーション層モジュール
///
/// ドメイン層のビジネスロジックとUI表示の橋渡しを行います。
/// Clean Architectureの依存方向に従い、プレゼンテーション層は
/// ドメイン層に依存しますが、その逆はありません。
///
/// # モジュール
/// - `input`: ユーザー入力処理
/// - `output`: コマンド結果の出力（人間向け・機械向け）
/// - `progress`: アップロード進捗のDTO変換

pub mod input;
pub mod output;
pub mod progress;
