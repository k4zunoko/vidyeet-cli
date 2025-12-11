/// ドメインサービス: タイムスタンプフォーマット
///
/// Unixタイムスタンプを人間向けの時刻文字列に変換する。
/// ドメイン層の責務として、ユーザー設定に基づいたビジネスルール(タイムゾーン変換)を適用する。
use crate::config::UserConfig;
use chrono::{DateTime, FixedOffset, TimeZone, Utc};

/// Unixタイムスタンプをユーザー設定に応じてフォーマット
///
/// # 引数
/// * `timestamp_str` - Unixタイムスタンプ（文字列、秒単位）
/// * `user_config` - ユーザー設定（タイムゾーンオフセットを含む）
///
/// # 戻り値
/// フォーマット済みの時刻文字列
/// - offset=0: "2024-12-01 14:30:45 +00:00" (UTC)
/// - offset=32400: "2024-12-01 23:30:45 +09:00" (JST)
/// - offset=-28800: "2024-12-01 06:30:45 -08:00" (PST)
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
        _ => return timestamp_str.to_string(), // 無効なUnixタイムスタンプの場合
    };

    // ユーザー設定のオフセットを適用
    format_with_offset(datetime_utc, user_config.timezone_offset_seconds)
}

/// 指定されたオフセット(秒)でフォーマット
fn format_with_offset(datetime: DateTime<Utc>, offset_seconds: i32) -> String {
    // オフセットを適用（無効な場合はUTCにフォールバック）
    let offset = FixedOffset::east_opt(offset_seconds)
        .unwrap_or_else(|| FixedOffset::east_opt(0).expect("UTC offset should always be valid"));
    
    let datetime_with_offset = datetime.with_timezone(&offset);
    datetime_with_offset.format("%Y-%m-%d %H:%M:%S %:z").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UserConfig;

    fn create_test_config(timezone_offset_seconds: i32) -> UserConfig {
        let mut config = UserConfig::default();
        config.timezone_offset_seconds = timezone_offset_seconds;
        config
    }

    #[test]
    fn test_format_timestamp_utc() {
        let config = create_test_config(0); // UTC = offset 0
        // 1764434950 = 2025-11-29 16:49:10 UTC
        let result = format_timestamp("1764434950", &config);
        assert!(result.contains("2025-11-29"));
        assert!(result.contains("+00:00")); // UTC offset format
    }

    #[test]
    fn test_format_timestamp_jst() {
        let config = create_test_config(32400); // JST = UTC+9 = 32400 seconds
        // 1764434950 = 2025-11-29 16:49:10 UTC = 2025-11-30 01:49:10 JST
        let result = format_timestamp("1764434950", &config);
        assert!(result.contains("2025-11-30"));
        assert!(result.contains("+09:00")); // JST offset format
    }

    #[test]
    fn test_format_timestamp_pst() {
        let config = create_test_config(-28800); // PST = UTC-8 = -28800 seconds
        // 1764434950 = 2025-11-29 16:49:10 UTC = 2025-11-29 08:49:10 PST
        let result = format_timestamp("1764434950", &config);
        assert!(result.contains("2025-11-29"));
        assert!(result.contains("-08:00")); // PST offset format
    }

    #[test]
    fn test_format_timestamp_invalid_timestamp() {
        let config = create_test_config(0);
        // 無効なUnixタイムスタンプは元の文字列を返す
        let result = format_timestamp("invalid", &config);
        assert_eq!(result, "invalid");
    }

    #[test]
    fn test_format_with_offset_utc() {
        let dt = Utc.timestamp_opt(1764434950, 0).unwrap();
        let result = format_with_offset(dt, 0);
        assert_eq!(result, "2025-11-29 16:49:10 +00:00");
    }

    #[test]
    fn test_format_with_offset_jst() {
        let dt = Utc.timestamp_opt(1764434950, 0).unwrap();
        let result = format_with_offset(dt, 32400); // JST = UTC+9
        // UTC 16:49:10 → JST 01:49:10 (+9時間、翌日)
        assert_eq!(result, "2025-11-30 01:49:10 +09:00");
    }

    #[test]
    fn test_format_with_offset_negative() {
        let dt = Utc.timestamp_opt(1764434950, 0).unwrap();
        let result = format_with_offset(dt, -18000); // EST = UTC-5
        assert_eq!(result, "2025-11-29 11:49:10 -05:00");
    }
}
