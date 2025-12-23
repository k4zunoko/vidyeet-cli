use crate::api::auth::AuthManager;
use crate::api::client::ApiClient;
use crate::commands::result::{CommandResult, DeleteResult};
use crate::config::{APP_CONFIG, UserConfig};
use anyhow::{Context, Result};

/// 削除コマンドを実行する
///
/// 指定されたアセットIDの動画をMux APIから削除します。
///
/// # 引数
/// * `asset_id` - 削除対象のアセットID
///
/// # 戻り値
/// 成功・失敗を示すResult<CommandResult>
///
/// # エラー
/// アプリケーション層としてanyhow::Resultを返し、
/// 設定・認証・インフラ層のエラーを集約します。
pub async fn execute(asset_id: &str) -> Result<CommandResult> {
    // ユーザー設定を読み込み
    let user_config = UserConfig::load()
        .context("Failed to load user configuration. Please check your config.toml file.")?;

    // 認証情報を取得
    let auth = user_config
        .get_auth()
        .context("Authentication credentials not found. Please run 'vidyeet login' first.")?;

    // 認証マネージャーとAPIクライアントを初期化
    let auth_manager = AuthManager::new(auth.token_id.clone(), auth.token_secret.clone());
    let client = ApiClient::new(APP_CONFIG.api.endpoint.to_string())
        .context("Failed to create API client")?;

    // アセットを削除
    delete_asset(&client, &auth_manager, asset_id)
        .await
        .context("Failed to delete asset")?;

    Ok(CommandResult::Delete(DeleteResult {
        asset_id: asset_id.to_string(),
    }))
}

/// Mux APIでアセットを削除
///
/// # 引数
/// * `client` - APIクライアント
/// * `auth_manager` - 認証マネージャー
/// * `asset_id` - 削除対象のアセットID
///
/// # 戻り値
/// 成功時は空のResult、失敗時はエラー
async fn delete_asset(
    client: &ApiClient,
    auth_manager: &AuthManager,
    asset_id: &str,
) -> Result<()> {
    let auth_header = auth_manager.get_auth_header();
    let endpoint = format!("/video/v1/assets/{}", asset_id);

    let response = client
        .delete(&endpoint, Some(&auth_header))
        .await
        .context(format!(
            "Failed to send DELETE request for asset {}",
            asset_id
        ))?;

    // 204 No Content が成功レスポンス
    if response.status() == 204 {
        Ok(())
    } else {
        // エラーレスポンスの処理
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        anyhow::bail!(
            "Failed to delete asset {}. Status: {}, Response: {}",
            asset_id,
            status,
            error_text
        )
    }
}
