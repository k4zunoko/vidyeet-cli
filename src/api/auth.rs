/// 認証マネージャー
///
/// Mux VideoのHTTP Basic認証を管理します。
/// Access Token IDとSecretを使用してHTTP Basic認証ヘッダーを生成します。
use crate::api::client::ApiClient;
use crate::api::error::InfraError;
use base64::{engine::general_purpose, Engine as _};

/// 認証マネージャー
pub struct AuthManager {
    token_id: String,
    token_secret: String,
}

impl AuthManager {
    /// 新しい認証マネージャーを作成
    ///
    /// # Arguments
    /// * `token_id` - Mux Access Token ID
    /// * `token_secret` - Mux Access Token Secret
    pub fn new(token_id: String, token_secret: String) -> Self {
        Self {
            token_id,
            token_secret,
        }
    }

    /// UserConfigから認証マネージャーを作成
    ///
    /// # Arguments
    /// * `token_id` - Mux Access Token ID
    /// * `token_secret` - Mux Access Token Secret
    pub fn from_credentials(token_id: String, token_secret: String) -> Self {
        Self::new(token_id, token_secret)
    }

    /// HTTP Basic認証ヘッダーの値を生成
    ///
    /// # Returns
    /// "Basic <base64(token_id:token_secret)>" 形式の文字列
    pub fn get_auth_header(&self) -> String {
        let credentials = format!("{}:{}", self.token_id, self.token_secret);
        let encoded = general_purpose::STANDARD.encode(credentials.as_bytes());
        format!("Basic {}", encoded)
    }

    /// 認証情報をテスト（GET /video/v1/assets で確認）
    ///
    /// # Returns
    /// 認証が成功すればOk、失敗すればErr
    pub async fn test_credentials(&self) -> Result<(), InfraError> {
        let client = ApiClient::production()?;
        let auth_header = self.get_auth_header();

        let response = client
            .get("/video/v1/assets", Some(&auth_header))
            .await?;

        ApiClient::check_response(response, "/video/v1/assets").await?;

        Ok(())
    }

    /// Token IDを取得（マスキング用）
    pub fn get_token_id(&self) -> &str {
        &self.token_id
    }

    /// Token IDをマスキングして表示
    pub fn get_masked_token_id(&self) -> String {
        if self.token_id.len() <= 8 {
            "*".repeat(self.token_id.len())
        } else {
            format!("{}***{}", &self.token_id[..4], &self.token_id[self.token_id.len()-4..])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_manager_creation() {
        let manager = AuthManager::new(
            "test_token_id".to_string(),
            "test_token_secret".to_string(),
        );
        assert_eq!(manager.token_id, "test_token_id");
        assert_eq!(manager.token_secret, "test_token_secret");
    }

    #[test]
    fn test_auth_header_generation() {
        let manager = AuthManager::new(
            "my_token_id".to_string(),
            "my_token_secret".to_string(),
        );

        let header = manager.get_auth_header();
        
        // "Basic " で始まることを確認
        assert!(header.starts_with("Basic "));
        
        // Base64デコードして元の値を確認
        let encoded = header.strip_prefix("Basic ").unwrap();
        let decoded = general_purpose::STANDARD.decode(encoded).unwrap();
        let decoded_str = String::from_utf8(decoded).unwrap();
        
        assert_eq!(decoded_str, "my_token_id:my_token_secret");
    }

    #[test]
    fn test_token_id_masking() {
        let manager = AuthManager::new(
            "abcdef123456789".to_string(),
            "secret".to_string(),
        );

        let masked = manager.get_masked_token_id();
        assert!(masked.contains("abcd"));
        assert!(masked.contains("***"));
        assert!(masked.contains("789"));
        assert!(!masked.contains("ef12345"));
    }

    #[test]
    fn test_short_token_id_masking() {
        let manager = AuthManager::new(
            "short".to_string(),
            "secret".to_string(),
        );

        let masked = manager.get_masked_token_id();
        assert_eq!(masked, "*****");
    }
}
