/// HTTPクライアント
///
/// api.videoとの通信を担当する汎用HTTPクライアント。
/// タイムアウト、エラーハンドリング、リトライロジックを含みます。
use crate::api::error::InfraError;
use crate::config::APP_CONFIG;
use reqwest::{Client, Response, StatusCode};
use std::time::Duration;

/// APIクライアント
pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    /// 新しいAPIクライアントを作成
    ///
    /// # Arguments
    /// * `base_url` - APIのベースURL（例: "https://ws.api.video"）
    ///
    /// # Returns
    /// 設定済みのAPIクライアント
    pub fn new(base_url: String) -> Result<Self, InfraError> {
        let timeout = Duration::from_secs(APP_CONFIG.api.timeout_seconds);

        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| InfraError::network(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, base_url })
    }

    /// デフォルトのプロダクション環境クライアントを作成
    pub fn production() -> Result<Self, InfraError> {
        Self::new(APP_CONFIG.api.endpoint.to_string())
    }

    /// POSTリクエストを送信
    ///
    /// # Arguments
    /// * `endpoint` - エンドポイントパス（例: "/auth/api-key"）
    /// * `body` - リクエストボディ（JSON）
    /// * `bearer_token` - Bearerトークン（オプション）
    pub async fn post<T: serde::Serialize>(
        &self,
        endpoint: &str,
        body: &T,
        bearer_token: Option<&str>,
    ) -> Result<Response, InfraError> {
        let url = format!("{}{}", self.base_url, endpoint);

        let mut request = self.client.post(&url).json(body);

        if let Some(token) = bearer_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                InfraError::Timeout {
                    operation: format!("POST {}", endpoint),
                }
            } else if e.is_connect() {
                InfraError::network(format!("Connection failed to {}: {}", url, e))
            } else {
                InfraError::network(format!("Request failed: {}", e))
            }
        })?;

        Ok(response)
    }

    /// レスポンスをチェックしてエラーを返す
    ///
    /// # Arguments
    /// * `response` - HTTPレスポンス
    /// * `endpoint` - エンドポイント名（エラーメッセージ用）
    pub async fn check_response(
        response: Response,
        endpoint: &str,
    ) -> Result<Response, InfraError> {
        let status = response.status();

        if status.is_success() {
            return Ok(response);
        }

        let status_code = status.as_u16();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to read error response".to_string());

        Err(InfraError::Api {
            endpoint: endpoint.to_string(),
            message: error_body,
            status_code: Some(status_code),
        })
    }

    /// JSONレスポンスをデシリアライズ
    pub async fn parse_json<T: serde::de::DeserializeOwned>(
        response: Response,
    ) -> Result<T, InfraError> {
        response.json().await.map_err(|e| {
            InfraError::network(format!("Failed to parse JSON response: {}", e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = ApiClient::new("https://ws.api.video".to_string());
        assert!(client.is_ok());
    }

    #[test]
    fn test_production_client() {
        let client = ApiClient::production();
        assert!(client.is_ok());
    }
}
