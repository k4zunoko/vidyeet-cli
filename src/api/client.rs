/// HTTPクライアント
///
/// Mux Videoとの通信を担当するHTTPクライアント。
/// タイムアウト、エラーハンドリング、HTTP Basic認証を含みます。
use crate::api::error::InfraError;
use crate::config::APP_CONFIG;
use reqwest::{Client, Response};
use std::time::Duration;

/// APIクライアントの結果型
type ApiResult<T> = Result<T, InfraError>;

/// APIクライアント
pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    /// 新しいAPIクライアントを作成
    ///
    /// # Arguments
    /// * `base_url` - APIのベースURL（例: "https://api.mux.com"）
    ///
    /// # Returns
    /// 設定済みのAPIクライアント
    pub fn new(base_url: String) -> ApiResult<Self> {
        let timeout = Duration::from_secs(APP_CONFIG.api.timeout_seconds);

        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| InfraError::network(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, base_url })
    }

    /// デフォルトのプロダクション環境クライアントを作成
    pub fn production() -> ApiResult<Self> {
        Self::new(APP_CONFIG.api.endpoint.to_string())
    }

    /// GETリクエストを送信
    ///
    /// # Arguments
    /// * `endpoint` - エンドポイントパス（例: "/video/v1/assets"）
    /// * `auth_header` - HTTP Basic認証ヘッダー（オプション）
    pub async fn get(
        &self,
        endpoint: &str,
        auth_header: Option<&str>,
    ) -> ApiResult<Response> {
        let url = self.build_url(endpoint);
        let request = self.build_request(self.client.get(&url), auth_header);
        
        Self::send_with_error_handling(request, endpoint, "GET").await
    }

    /// POSTリクエストを送信
    ///
    /// # Arguments
    /// * `endpoint` - エンドポイントパス（例: "/video/v1/uploads"）
    /// * `body` - リクエストボディ（JSON）
    /// * `auth_header` - HTTP Basic認証ヘッダー（オプション）
    pub async fn post<T: serde::Serialize>(
        &self,
        endpoint: &str,
        body: &T,
        auth_header: Option<&str>,
    ) -> ApiResult<Response> {
        let url = self.build_url(endpoint);
        let request = self.build_request(self.client.post(&url).json(body), auth_header);
        
        Self::send_with_error_handling(request, endpoint, "POST").await
    }

    /// PUTリクエストを送信（ファイルアップロード用）
    ///
    /// # Arguments
    /// * `url` - 完全なURL（Mux Direct UploadのURL）
    /// * `body` - アップロードするデータ（バイト列）
    /// * `content_type` - Content-Typeヘッダー
    pub async fn put(
        &self,
        url: &str,
        body: Vec<u8>,
        content_type: &str,
    ) -> Result<Response, InfraError> {
        let response = self
            .client
            .put(url)
            .header("Content-Type", content_type)
            .body(body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    InfraError::Timeout {
                        operation: format!("PUT {}", url),
                    }
                } else if e.is_connect() {
                    InfraError::network(format!("Connection failed to {}: {}", url, e))
                } else {
                    InfraError::network(format!("Request failed: {}", e))
                }
            })?;

        Ok(response)
    }

    /// DELETEリクエストを送信
    ///
    /// # Arguments
    /// * `endpoint` - エンドポイントパス（例: "/video/v1/assets/{ASSET_ID}"）
    /// * `auth_header` - HTTP Basic認証ヘッダー（オプション）
    pub async fn delete(
        &self,
        endpoint: &str,
        auth_header: Option<&str>,
    ) -> ApiResult<Response> {
        let url = self.build_url(endpoint);
        let request = self.build_request(self.client.delete(&url), auth_header);
        
        Self::send_with_error_handling(request, endpoint, "DELETE").await
    }

    /// URLを構築
    fn build_url(&self, endpoint: &str) -> String {
        format!("{}{}", self.base_url, endpoint)
    }

    /// 認証ヘッダーを付与したリクエストを構築
    fn build_request(
        &self,
        mut request: reqwest::RequestBuilder,
        auth_header: Option<&str>,
    ) -> reqwest::RequestBuilder {
        if let Some(auth) = auth_header {
            request = request.header("Authorization", auth);
        }
        request
    }

    /// リクエストを送信し、エラーハンドリングを行う
    async fn send_with_error_handling(
        request: reqwest::RequestBuilder,
        endpoint: &str,
        method: &str,
    ) -> ApiResult<Response> {
        request.send().await.map_err(|e| {
            if e.is_timeout() {
                InfraError::timeout(format!("{} {}", method, endpoint))
            } else if e.is_connect() {
                InfraError::network(format!("Connection failed for {} {}: {}", method, endpoint, e))
            } else {
                InfraError::network(format!("Request failed for {} {}: {}", method, endpoint, e))
            }
        })
    }

    /// レスポンスをチェックしてエラーを返す
    ///
    /// # Arguments
    /// * `response` - HTTPレスポンス
    /// * `endpoint` - エンドポイント名（エラーメッセージ用）
    pub async fn check_response(
        response: Response,
        endpoint: &str,
    ) -> ApiResult<Response> {
        let status = response.status();

        if status.is_success() {
            return Ok(response);
        }

        let status_code = status.as_u16();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to read error response".to_string());

        Err(InfraError::api(endpoint, error_body, Some(status_code)))
    }

    /// JSONレスポンスをデシリアライズ
    pub async fn parse_json<T: serde::de::DeserializeOwned>(
        response: Response,
    ) -> ApiResult<T> {
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
        let client = ApiClient::new("https://api.mux.com".to_string());
        assert!(client.is_ok());
    }

    #[test]
    fn test_production_client() {
        let client = ApiClient::production();
        assert!(client.is_ok());
    }
}
