use anyhow::{Context, Result};
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};

/// Parse a time specification into a DateTime<Utc>
///
/// Supports:
/// - "now" keyword
/// - ISO8601: "2024-10-19T14:23:45Z"
/// - Short date: "2024-10-19" (defaults to 00:00:00 UTC)
/// - Unix timestamp: "@1697328000"
/// - Relative time: "7 days ago", "1 week ago", "yesterday"
pub fn parse_time_spec(spec: &str) -> Result<DateTime<Utc>> {
    let spec = spec.trim();

    // 1. "now" keyword
    if spec.eq_ignore_ascii_case("now") {
        return Ok(Utc::now());
    }

    // 2. Unix timestamp: @1234567890
    if let Some(ts_str) = spec.strip_prefix('@') {
        let timestamp: i64 = ts_str.parse().context("Invalid unix timestamp format")?;
        return DateTime::from_timestamp(timestamp, 0).context("Unix timestamp out of valid range");
    }

    // 3. ISO8601: 2024-10-19T14:23:45Z
    if let Ok(dt) = DateTime::parse_from_rfc3339(spec) {
        return Ok(dt.with_timezone(&Utc));
    }

    // 4. Short date: 2024-10-19
    if let Ok(date) = NaiveDate::parse_from_str(spec, "%Y-%m-%d") {
        return Utc
            .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .context("Ambiguous date/time");
    }

    // 5. Relative time: "7 days ago", "1 week ago", "yesterday"
    parse_relative_time(spec)
}

fn parse_relative_time(spec: &str) -> Result<DateTime<Utc>> {
    let now = Utc::now();
    let spec_lower = spec.to_lowercase();

    // Special keywords
    match spec_lower.as_str() {
        "yesterday" => {
            let yesterday = now - Duration::days(1);
            let date = yesterday.date_naive();
            return Ok(Utc
                .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
                .single()
                .unwrap());
        }
        "today" => {
            let date = now.date_naive();
            return Ok(Utc
                .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
                .single()
                .unwrap());
        }
        _ => {}
    }

    // Pattern: "N units ago"
    let parts: Vec<&str> = spec_lower.split_whitespace().collect();
    if parts.len() == 3 && parts[2] == "ago" {
        let num: i64 = parts[0]
            .parse()
            .context("Invalid number in relative time")?;

        let duration = match parts[1].trim_end_matches('s') {
            "second" => Duration::seconds(num),
            "minute" => Duration::minutes(num),
            "hour" => Duration::hours(num),
            "day" => Duration::days(num),
            "week" => Duration::weeks(num),
            "month" => Duration::days(num * 30), // Approximate
            "year" => Duration::days(num * 365), // Approximate
            unit => {
                return Err(anyhow::anyhow!("Unknown time unit: {unit}"));
            }
        };

        return Ok(now - duration);
    }

    Err(anyhow::anyhow!(
        "Invalid time specification: '{spec}'. Expected formats: 'now', '2024-10-19', '7 days ago', '@1234567890'"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_parse_now() {
        let result = parse_time_spec("now");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_iso8601() {
        let result = parse_time_spec("2024-10-19T14:23:45Z");
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 10);
        assert_eq!(dt.day(), 19);
    }

    #[test]
    fn test_parse_short_date() {
        let result = parse_time_spec("2024-10-19");
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 10);
        assert_eq!(dt.day(), 19);
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
    }

    #[test]
    fn test_parse_unix_timestamp() {
        let result = parse_time_spec("@1697328000");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_relative_days() {
        let result = parse_time_spec("7 days ago");
        assert!(result.is_ok());
        let dt = result.unwrap();
        let expected = Utc::now() - Duration::days(7);
        // Allow 1 second tolerance
        assert!((dt.timestamp() - expected.timestamp()).abs() < 2);
    }

    #[test]
    fn test_parse_yesterday() {
        let result = parse_time_spec("yesterday");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_invalid() {
        let result = parse_time_spec("invalid input");
        assert!(result.is_err());
    }
}
