/// api.video APIクライアントモジュール
///
/// api.videoとの通信を担当するモジュール。
/// 認証、動画アップロード、動画管理機能を提供します。
pub mod auth;
pub mod client;
pub mod error;
pub mod types;

pub use auth::AuthManager;
pub use client::ApiClient;
pub use error::InfraError;
pub use types::TokenResponse;
