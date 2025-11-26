/// 認証マネージャー
///
/// api.videoの認証フローを管理します。
/// - APIキーからトークン取得
/// - リフレッシュトークンによるトークン更新
/// - メモリ内でのアクセストークン管理
use crate::api::client::ApiClient;
use crate::api::error::InfraError;
use crate::api::types::TokenResponse;
use std::time::{Duration, Instant};

/// 認証マネージャー
pub struct AuthManager {
    client: ApiClient,
    access_token: Option<String>,
    token_expires_at: Option<Instant>,
}

impl AuthManager {
    /// 新しい認証マネージャーを作成
    pub fn new() -> Result<Self, InfraError> {
        let client = ApiClient::production()?;

        Ok(Self {
            client,
            access_token: None,
            token_expires_at: None,
        })
    }

    /// APIキーからリフレッシュトークンを取得（初回ログイン）
    ///
    /// # Arguments
    /// * `api_key` - api.videoのAPIキー
    ///
    /// # Returns
    /// リフレッシュトークン（config.tomlに保存する）
    pub async fn login(&mut self, api_key: &str) -> Result<String, InfraError> {
        let request_body = serde_json::json!({
            "apiKey": api_key
        });

        let response = self.client.post("/auth/api-key", &request_body, None).await?;

        let response = ApiClient::check_response(response, "/auth/api-key").await?;

        let token_response: TokenResponse = ApiClient::parse_json(response).await?;

        if !token_response.is_valid() {
            return Err(InfraError::Api {
                endpoint: "/auth/api-key".to_string(),
                message: "Invalid token response from server".to_string(),
                status_code: None,
            });
        }

        // アクセストークンをメモリに保持
        self.access_token = Some(token_response.access_token.clone());
        self.token_expires_at =
            Some(Instant::now() + Duration::from_secs(token_response.expires_in));

        // リフレッシュトークンを返却（呼び出し側がconfig.tomlに保存）
        Ok(token_response.refresh_token)
    }

    /// リフレッシュトークンからアクセストークンを取得
    ///
    /// # Arguments
    /// * `refresh_token` - config.tomlから読み込んだリフレッシュトークン
    ///
    /// # Returns
    /// (アクセストークン, 新しいリフレッシュトークン)のタプル
    pub async fn get_access_token(
        &mut self,
        refresh_token: &str,
    ) -> Result<(String, String), InfraError> {
        // トークンが有効な場合は既存のものを使用
        if let Some(token) = &self.access_token {
            if !self.is_token_expired() {
                return Ok((token.clone(), refresh_token.to_string()));
            }
        }

        // トークンを更新
        self.refresh_access_token(refresh_token).await
    }

    /// リフレッシュトークンでアクセストークンを更新
    async fn refresh_access_token(
        &mut self,
        refresh_token: &str,
    ) -> Result<(String, String), InfraError> {
        let request_body = serde_json::json!({
            "refreshToken": refresh_token
        });

        let response = self
            .client
            .post("/auth/refresh", &request_body, None)
            .await?;

        let response = ApiClient::check_response(response, "/auth/refresh").await?;

        let token_response: TokenResponse = ApiClient::parse_json(response).await?;

        if !token_response.is_valid() {
            return Err(InfraError::Api {
                endpoint: "/auth/refresh".to_string(),
                message: "Invalid token response from server".to_string(),
                status_code: None,
            });
        }

        // アクセストークンをメモリに保持
        self.access_token = Some(token_response.access_token.clone());
        self.token_expires_at =
            Some(Instant::now() + Duration::from_secs(token_response.expires_in));

        // (アクセストークン, 新しいリフレッシュトークン)を返却
        Ok((
            token_response.access_token,
            token_response.refresh_token,
        ))
    }

    /// トークンが期限切れかチェック（メモリ内）
    ///
    /// トークンの有効期限が切れているか、または5分以内に切れる場合にtrueを返します。
    fn is_token_expired(&self) -> bool {
        match self.token_expires_at {
            Some(expires_at) => {
                // 5分の余裕を持たせて期限切れと判定
                let grace_period = Duration::from_secs(300);
                let now = Instant::now();
                
                // expires_atより前に期限切れと判定する時刻
                let effective_expiration = expires_at.checked_sub(grace_period)
                    .unwrap_or(expires_at);
                
                now >= effective_expiration
            }
            std::option::Option::None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_manager_creation() {
        let manager = AuthManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_token_expiry_check() {
        let mut manager = AuthManager::new().unwrap();

        // 初期状態では期限切れ
        assert!(manager.is_token_expired());

        // トークンを設定
        manager.access_token = Some("test_token".to_string());
        manager.token_expires_at = Some(Instant::now() + Duration::from_secs(3600));

        // 有効期限内
        assert!(!manager.is_token_expired());

        // 期限切れのトークン
        manager.token_expires_at = Some(Instant::now() - Duration::from_secs(1));
        assert!(manager.is_token_expired());
    }
}
