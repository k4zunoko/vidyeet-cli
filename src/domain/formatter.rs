/// ドメインサービス: タイムスタンプフォーマット
///
/// Unixタイムスタンプを人間向けの時刻文字列に変換する。
/// ドメイン層の責務として、ユーザー設定に基づいたビジネスルール（タイムゾーン変換）を適用する。
use crate::config::UserConfig;
use chrono::{DateTime, Local, TimeZone, Utc};

/// Unixタイムスタンプをユーザー設定に応じてフォーマット
///
/// # 引数
/// * `timestamp_str` - Unixタイムスタンプ（文字列、秒単位）
/// * `user_config` - ユーザー設定（タイムゾーン設定を含む）
///
/// # 戻り値
/// フォーマット済みの時刻文字列
/// - UTC: "2024-12-01 14:30:45 UTC"
/// - JST: "2024-12-01 23:30:45 JST"
/// - Local: "2024-12-01 23:30:45 +09:00" (システムのローカルタイムゾーン)
///
/// パースエラーの場合は、元の文字列をそのまま返します。
pub fn format_timestamp(timestamp_str: &str, user_config: &UserConfig) -> String {
    // Unixタイムスタンプをパース
    let timestamp = match timestamp_str.parse::<i64>() {
        Ok(ts) => ts,
        Err(_) => return timestamp_str.to_string(), // パースエラー時は元の文字列を返す
    };

    // UTC DateTimeに変換
    let datetime_utc = match Utc.timestamp_opt(timestamp, 0) {
        chrono::LocalResult::Single(dt) => dt,
        _ => return timestamp_str.to_string(), // 無効なタイムスタンプの場合
    };

    // ユーザー設定のタイムゾーンに応じてフォーマット
    match user_config.timezone.as_str() {
        "UTC" => format_utc(datetime_utc),
        "JST" => format_jst(datetime_utc),
        "Local" => format_local(datetime_utc),
        _ => format_utc(datetime_utc), // デフォルトはUTC
    }
}

/// UTC形式でフォーマット
fn format_utc(datetime: DateTime<Utc>) -> String {
    datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// JST (UTC+9) 形式でフォーマット
fn format_jst(datetime: DateTime<Utc>) -> String {
    // JSTはUTC+9時間
    let jst_offset = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
    let datetime_jst = datetime.with_timezone(&jst_offset);
    datetime_jst.format("%Y-%m-%d %H:%M:%S JST").to_string()
}

/// ローカルタイムゾーン形式でフォーマット
fn format_local(datetime: DateTime<Utc>) -> String {
    let datetime_local: DateTime<Local> = datetime.with_timezone(&Local);
    datetime_local.format("%Y-%m-%d %H:%M:%S %z").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UserConfig;

    fn create_test_config(timezone: &str) -> UserConfig {
        let mut config = UserConfig::default();
        config.timezone = timezone.to_string();
        config
    }

    #[test]
    fn test_format_timestamp_utc() {
        let config = create_test_config("UTC");
        // 1764434950 = 2025-11-29 16:49:10 UTC
        let result = format_timestamp("1764434950", &config);
        assert!(result.contains("2025-11-29"));
        assert!(result.contains("UTC"));
    }

    #[test]
    fn test_format_timestamp_jst() {
        let config = create_test_config("JST");
        // 1764434950 = 2025-11-29 16:49:10 UTC = 2025-11-30 01:49:10 JST
        let result = format_timestamp("1764434950", &config);
        assert!(result.contains("2025-11-30"));
        assert!(result.contains("JST"));
    }

    #[test]
    fn test_format_timestamp_local() {
        let config = create_test_config("Local");
        let result = format_timestamp("1764434950", &config);
        assert!(result.contains("2025-11"));
        // ローカルタイムゾーンオフセットが含まれる（例: +09:00）
        assert!(result.contains("+") || result.contains("-"));
    }

    #[test]
    fn test_format_timestamp_invalid_timestamp() {
        let config = create_test_config("UTC");
        // 無効なタイムスタンプは元の文字列を返す
        let result = format_timestamp("invalid", &config);
        assert_eq!(result, "invalid");
    }

    #[test]
    fn test_format_timestamp_unsupported_timezone() {
        let config = create_test_config("INVALID");
        // サポートされていないタイムゾーンはUTCとして扱う
        let result = format_timestamp("1733066445", &config);
        assert!(result.contains("UTC"));
    }

    #[test]
    fn test_format_utc() {
        let dt = Utc.timestamp_opt(1764434950, 0).unwrap();
        let result = format_utc(dt);
        assert_eq!(result, "2025-11-29 16:49:10 UTC");
    }

    #[test]
    fn test_format_jst() {
        let dt = Utc.timestamp_opt(1764434950, 0).unwrap();
        let result = format_jst(dt);
        // UTC 16:49:10 → JST 01:49:10 (+9時間、翌日)
        assert_eq!(result, "2025-11-30 01:49:10 JST");
    }
}
