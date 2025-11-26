/// API通信用の型定義
///
/// api.videoのAPIレスポンスをデシリアライズするための構造体を定義します。
use serde::{Deserialize, Serialize};

/// 認証トークンレスポンス
///
/// POST /auth/api-key および POST /auth/refresh のレスポンス型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    /// トークンタイプ（常に "Bearer"）
    pub token_type: String,

    /// アクセストークンの有効期限（秒）
    pub expires_in: u64,

    /// アクセストークン（API呼び出し用）
    pub access_token: String,

    /// リフレッシュトークン（トークン更新用）
    pub refresh_token: String,
}

impl TokenResponse {
    /// トークンレスポンスが有効かチェック
    pub fn is_valid(&self) -> bool {
        !self.access_token.is_empty()
            && !self.refresh_token.is_empty()
            && self.token_type == "Bearer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_response_deserialization() {
        let json = r#"{
            "token_type": "Bearer",
            "expires_in": 3600,
            "access_token": "test_access_token",
            "refresh_token": "test_refresh_token"
        }"#;

        let response: TokenResponse = serde_json::from_str(json).expect("Failed to parse");

        assert_eq!(response.token_type, "Bearer");
        assert_eq!(response.expires_in, 3600);
        assert_eq!(response.access_token, "test_access_token");
        assert_eq!(response.refresh_token, "test_refresh_token");
        assert!(response.is_valid());
    }

    #[test]
    fn test_token_response_invalid() {
        let response = TokenResponse {
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            access_token: "".to_string(),
            refresh_token: "test".to_string(),
        };

        assert!(!response.is_valid());
    }
}
