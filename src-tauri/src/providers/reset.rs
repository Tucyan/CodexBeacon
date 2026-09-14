use super::snapshot;
use serde_json::{json, Map, Value};
use std::time::{SystemTime, UNIX_EPOCH};

const BASE_HOST: &str = "codex-resets.com";
const STATUS_PATH: &str = "/api/v1/status";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResetError {
    Unsupported,
    Timeout,
    Network,
    RateLimited,
    ServiceUnavailable,
    Http(u16),
    InvalidResponse,
    NotModifiedWithoutCache,
}

impl ResetError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported_platform",
            Self::Timeout => "timeout",
            Self::Network => "network_error",
            Self::RateLimited => "rate_limited",
            Self::ServiceUnavailable => "service_unavailable",
            Self::Http(_) => "http_error",
            Self::InvalidResponse | Self::NotModifiedWithoutCache => "invalid_response",
        }
    }
}

#[derive(Debug, Default, Clone)]
struct HttpResponse {
    status: u16,
    body: Vec<u8>,
    etag: Option<String>,
    retry_after: Option<String>,
}

/// A persistent read-only reset client. Keep one instance in the root worker
/// so ETag validation can turn a 304 into the last validated snapshot.
#[derive(Debug, Default)]
pub struct ResetClient {
    etag: Option<String>,
    cached: Option<Value>,
    retry_after_ms: Option<u64>,
}

impl ResetClient {
    pub fn new() -> Self { Self::default() }

    pub fn fetch_status(&mut self) -> Result<Value, ResetError> {
        let response = http_get_status(self.etag.as_deref())?;
        self.retry_after_ms = None;
        match response.status {
            200 => {
                let value: Value = serde_json::from_slice(&response.body).map_err(|_| ResetError::InvalidResponse)?;
                validate_status(&value)?;
                self.etag = response.etag.filter(|value| !value.chars().any(|ch| ch == '\r' || ch == '\n') && value.len() <= 512);
                self.cached = Some(value.clone());
                Ok(value)
            }
            304 => {
                if let Some(value) = response.etag.filter(|value| !value.chars().any(|ch| ch == '\r' || ch == '\n') && value.len() <= 512) {
                    self.etag = Some(value);
                }
                self.cached.clone().ok_or(ResetError::NotModifiedWithoutCache)
            }
            429 => {
                self.retry_after_ms = response.retry_after.as_deref().and_then(parse_retry_after_ms);
                Err(ResetError::RateLimited)
            }
            503 => Err(ResetError::ServiceUnavailable),
            status => Err(ResetError::Http(status)),
        }
    }

    /// Converts a validated API envelope to the small public data shape.
    pub fn fetch_snapshot(&mut self) -> crate::model::ProviderSnapshot {
        match self.fetch_status() {
            Ok(value) => snapshot("ready", Some(now_seconds()), None, Some(sanitize_status(&value))),
            Err(error) => snapshot("error", None, Some(error.code()), None),
        }
    }

    pub fn etag(&self) -> Option<&str> { self.etag.as_deref() }

    /// Seconds from Retry-After, retained for the root scheduler. The client
    /// itself never sleeps or retries.
    pub fn retry_after_ms(&self) -> Option<u64> { self.retry_after_ms }
}

pub fn read_reset() -> crate::model::ProviderSnapshot {
    ResetClient::new().fetch_snapshot()
}

fn now_seconds() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|duration| duration.as_secs_f64()).unwrap_or(0.0)
}

fn parse_retry_after_ms(value: &str) -> Option<u64> {
    let seconds = value.trim().parse::<f64>().ok()?;
    if !seconds.is_finite() || seconds < 0.0 { return None; }
    Some((seconds * 1000.0).round() as u64)
}

fn required_keys(value: &Map<String, Value>, keys: &[&str]) -> bool {
    keys.iter().all(|key| value.contains_key(*key))
}

fn nonempty_string(value: Option<&Value>) -> bool { value.and_then(Value::as_str).is_some_and(|s| !s.is_empty()) }

fn text_string(value: Option<&Value>) -> bool { value.and_then(Value::as_str).is_some() }

/// RFC3339 date-time with an explicit timezone. This avoids adding a date
/// dependency while enforcing the API's important boundary (no local times).
pub fn valid_datetime(value: Option<&Value>) -> bool {
    let Some(text) = value.and_then(Value::as_str) else { return false };
    if text.len() < 20 { return false; }
    let bytes = text.as_bytes();
    let punctuation = [(4, b'-'), (7, b'-'), (10, b'T'), (13, b':'), (16, b':')];
    if punctuation.iter().any(|(index, expected)| bytes.get(*index) != Some(expected)) { return false; }
    if ![0..4, 5..7, 8..10, 11..13, 14..16, 17..19].iter().all(|range| text.as_bytes().get(range.clone()).is_some_and(|part| part.iter().all(u8::is_ascii_digit))) { return false; }
    let year = text[0..4].parse::<u32>().ok();
    let month = text[5..7].parse::<u32>().ok();
    let day = text[8..10].parse::<u32>().ok();
    let hour = text[11..13].parse::<u32>().ok();
    let minute = text[14..16].parse::<u32>().ok();
    let second = text[17..19].parse::<u32>().ok();
    let Some((year, month, day, hour, minute, second)) = year.zip(month).zip(day).zip(hour).zip(minute).zip(second).map(|(((((year, month), day), hour), minute), second)| (year, month, day, hour, minute, second)) else { return false };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if !(1..=12).contains(&month) || day == 0 || day > month_days[(month - 1) as usize] || hour > 23 || minute > 59 || second > 59 { return false; }
    let zone_start = if text.ends_with('Z') { text.len() - 1 } else { text.len().checked_sub(6).filter(|index| matches!(bytes.get(*index), Some(b'+') | Some(b'-'))).unwrap_or(0) };
    if zone_start < 19 { return false; }
    if zone_start > 19 && (bytes.get(19) != Some(&b'.') || !bytes[20..zone_start].iter().all(u8::is_ascii_digit)) { return false; }
    if text.ends_with('Z') { return true; }
    let offset = &text[zone_start..];
    offset.as_bytes().get(3) == Some(&b':')
        && offset[1..3].parse::<u8>().is_ok_and(|value| value <= 23)
        && offset[4..6].parse::<u8>().is_ok_and(|value| value <= 59)
}

fn valid_uri(value: Option<&Value>) -> bool {
    value.and_then(Value::as_str).is_some_and(|uri| {
        (uri.starts_with("https://") || uri.starts_with("http://"))
            && uri[uri.find("://").unwrap_or(0) + 3..].chars().all(|ch| !ch.is_whitespace())
            && uri[uri.find("://").unwrap_or(0) + 3..].contains('.')
    })
}

fn validate_source(value: Option<&Value>) -> bool {
    let Some(source) = value.and_then(Value::as_object) else { return false };
    match source.get("type").and_then(Value::as_str) {
        Some("x_post") => required_keys(source, &["type", "author", "url"])
            && source.get("author").and_then(Value::as_str) == Some("thsottiaux") && valid_uri(source.get("url")),
        Some("observed") => required_keys(source, &["type"])
            && source.get("url").map_or(true, |url| valid_uri(Some(url))),
        _ => false,
    }
}

fn validate_reset(value: Option<&Value>) -> bool {
    let Some(reset) = value.and_then(Value::as_object) else { return false };
    required_keys(reset, &["id", "reset_type", "announced_at", "text", "source"])
        && nonempty_string(reset.get("id"))
        && reset.get("reset_type").and_then(Value::as_str).is_some_and(|kind| kind == "regular" || kind == "banked")
        && valid_datetime(reset.get("announced_at"))
        && text_string(reset.get("text"))
        && validate_source(reset.get("source"))
}

fn validate_watch(value: Option<&Value>) -> bool {
    let Some(watch) = value.and_then(Value::as_object) else { return false };
    let chance_ok = watch.get("reset_chance_percent").is_some_and(|chance| chance.is_null() || chance.as_i64().is_some_and(|n| (0..=100).contains(&n)));
    required_keys(watch, &["level", "reset_chance_percent", "forecast_window", "observed_at", "expires_at", "text", "source"])
        && watch.get("level").and_then(Value::as_str).is_some_and(|level| level == "elevated" || level == "strong")
        && chance_ok && text_string(watch.get("forecast_window"))
        && valid_datetime(watch.get("observed_at")) && valid_datetime(watch.get("expires_at"))
        && text_string(watch.get("text")) && validate_source(watch.get("source"))
}

fn validate_stats(value: Option<&Value>) -> bool {
    let Some(stats) = value.and_then(Value::as_object) else { return false };
    let number = |key: &str| stats.get(key).is_some_and(|item| item.is_null() || item.as_f64().is_some_and(|n| n.is_finite() && n >= 0.0));
    required_keys(stats, &["total", "last_reset_at", "days_since_last", "avg_interval_days"])
        && stats.get("total").and_then(Value::as_i64).is_some_and(|n| n >= 0)
        && (stats.get("last_reset_at").is_some_and(Value::is_null) || valid_datetime(stats.get("last_reset_at")))
        && number("days_since_last") && number("avg_interval_days")
}

fn validate_meta(value: Option<&Value>) -> bool {
    let Some(meta) = value.and_then(Value::as_object) else { return false };
    required_keys(meta, &["api_version", "generated_at"])
        && meta.get("api_version").and_then(Value::as_str) == Some("v1")
        && valid_datetime(meta.get("generated_at"))
}

pub fn validate_status(value: &Value) -> Result<(), ResetError> {
    let Some(root) = value.as_object() else { return Err(ResetError::InvalidResponse) };
    let Some(data) = root.get("data").and_then(Value::as_object) else { return Err(ResetError::InvalidResponse) };
    if !required_keys(root, &["data", "meta"]) || !required_keys(data, &["latest_reset", "active_watch", "stats"])
        || (!data.get("latest_reset").is_some_and(Value::is_null) && !validate_reset(data.get("latest_reset")))
        || (!data.get("active_watch").is_some_and(Value::is_null) && !validate_watch(data.get("active_watch")))
        || !validate_stats(data.get("stats")) || !validate_meta(root.get("meta")) {
        return Err(ResetError::InvalidResponse);
    }
    Ok(())
}

pub fn sanitize_status(value: &Value) -> Value {
    let latest = value.get("data").and_then(|v| v.get("latest_reset"));
    let latest_reset = latest.filter(|v| !v.is_null()).map(|reset| json!({
        "id": reset.get("id").and_then(Value::as_str),
        "announcedAt": reset.get("announced_at").and_then(Value::as_str),
        "resetType": reset.get("reset_type").and_then(Value::as_str),
        "sourceType": reset.get("source").and_then(|source| source.get("type")).and_then(Value::as_str),
    })).unwrap_or(Value::Null);
    let active_watch = value.get("data").and_then(|v| v.get("active_watch"));
    let active_watch = active_watch.filter(|v| !v.is_null()).map(|watch| json!({
        "level": watch.get("level").and_then(Value::as_str),
        "chancePercent": watch.get("reset_chance_percent"),
        "forecastWindow": watch.get("forecast_window").and_then(Value::as_str),
        "observedAt": watch.get("observed_at").and_then(Value::as_str),
        "expiresAt": watch.get("expires_at").and_then(Value::as_str),
    })).unwrap_or(Value::Null);
    json!({"latestReset": latest_reset, "activeWatch": active_watch})
}

#[cfg(windows)]
fn http_get_status(etag: Option<&str>) -> Result<HttpResponse, ResetError> {
    use std::ffi::c_void;
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Networking::WinHttp::*;

    fn wide(text: &str) -> Vec<u16> { text.encode_utf16().chain(Some(0)).collect() }
    unsafe fn close(handle: *mut c_void) { if !handle.is_null() { WinHttpCloseHandle(handle); } }
    unsafe fn query_text(request: *mut c_void, info: u32) -> Option<String> {
        let mut size = 0_u32;
        let _ = WinHttpQueryHeaders(request, info, null(), null_mut(), &mut size, null_mut());
        if size == 0 { return None; }
        let mut buffer = vec![0_u16; (size as usize + 1) / 2];
        if WinHttpQueryHeaders(request, info, null(), buffer.as_mut_ptr() as *mut c_void, &mut size, null_mut()) == 0 { return None; }
        Some(String::from_utf16_lossy(&buffer[..buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len())]))
    }

    unsafe {
        let agent = wide("DesktopDashboard/0.1");
        let session = WinHttpOpen(agent.as_ptr(), WINHTTP_ACCESS_TYPE_DEFAULT_PROXY, null(), null(), 0);
        if session.is_null() { return Err(ResetError::Network); }
        let result = (|| {
            let deadline=std::time::Instant::now()+std::time::Duration::from_secs(10);
            if WinHttpSetTimeouts(session, 5000, 5000, 5000, 5000) == 0 { return Err(ResetError::Timeout); }
            let host = wide(BASE_HOST);
            let connection = WinHttpConnect(session, host.as_ptr(), 443, 0);
            if connection.is_null() { return Err(ResetError::Network); }
            let path = wide(STATUS_PATH);
            let verb = wide("GET");
            let request = WinHttpOpenRequest(connection, verb.as_ptr(), path.as_ptr(), null(), null(), null(), WINHTTP_FLAG_SECURE);
            if request.is_null() { close(connection); return Err(ResetError::Network); }
            let headers = etag.map(|value| wide(&format!("If-None-Match: {value}\r\n")));
            let header_ptr = headers.as_ref().map_or(null(), |value| value.as_ptr());
            if WinHttpSendRequest(request, header_ptr, u32::MAX, null(), 0, 0, 0) == 0 || WinHttpReceiveResponse(request, null_mut()) == 0 {
                close(request); close(connection); return Err(ResetError::Network);
            }
            let status = query_text(request, WINHTTP_QUERY_STATUS_CODE).and_then(|text| text.parse().ok()).unwrap_or(0);
            let etag = query_text(request, WINHTTP_QUERY_ETAG);
            let retry_after = query_text(request, WINHTTP_QUERY_RETRY_AFTER);
            let mut body = Vec::new();
            if status == 200 {
                loop {
                    if std::time::Instant::now()>=deadline{close(request);close(connection);return Err(ResetError::Timeout);}
                    let mut available = 0_u32;
                    if WinHttpQueryDataAvailable(request, &mut available) == 0 {close(request);close(connection);return Err(ResetError::Network);}
                    if available == 0 { break; }
                    if body.len().saturating_add(available as usize)>1024*1024{close(request);close(connection);return Err(ResetError::InvalidResponse);}
                    let mut chunk = vec![0_u8; available as usize];
                    let mut read = 0_u32;
                    if WinHttpReadData(request, chunk.as_mut_ptr() as *mut c_void, available, &mut read) == 0 {close(request);close(connection);return Err(ResetError::Network);}
                    body.extend_from_slice(&chunk[..read as usize]);
                }
            }
            close(request); close(connection);
            Ok(HttpResponse { status, body, etag, retry_after })
        })();
        close(session);
        result
    }
}

#[cfg(not(windows))]
fn http_get_status(_etag: Option<&str>) -> Result<HttpResponse, ResetError> { Err(ResetError::Unsupported) }

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        json!({"data":{"latest_reset":{"id":"1","reset_type":"regular","announced_at":"2026-09-03T10:00:00+02:00","text":"","source":{"type":"observed"}},"active_watch":null,"stats":{"total":1,"last_reset_at":"2026-09-03T10:00:00+02:00","days_since_last":0,"avg_interval_days":null}},"meta":{"api_version":"v1","generated_at":"2026-09-03T10:01:00Z"}})
    }
    #[test]
    fn strict_status_validation_and_sanitization() {
        let value = fixture();
        assert!(validate_status(&value).is_ok());
        assert_eq!(sanitize_status(&value), json!({"latestReset":{"id":"1","announcedAt":"2026-09-03T10:00:00+02:00","resetType":"regular","sourceType":"observed"},"activeWatch":null}));
        let mut bad = value;
        bad["data"]["latest_reset"]["source"] = Value::String("observed".into());
        assert_eq!(validate_status(&bad), Err(ResetError::InvalidResponse));
    }
    #[test]
    fn sanitizes_banked_x_post_and_watch_without_post_details() {
        let mut value = fixture();
        value["data"]["latest_reset"]["reset_type"] = json!("banked");
        value["data"]["latest_reset"]["source"] = json!({"type":"x_post","author":"thsottiaux","url":"https://x.com/thsottiaux/status/1"});
        value["data"]["active_watch"] = json!({
            "level":"strong", "reset_chance_percent":null, "forecast_window":"by Thursday",
            "observed_at":"2026-09-03T09:00:00Z", "expires_at":"2026-09-04T09:00:00+00:00",
            "text":"private forecast text", "source":{"type":"observed","url":"https://example.test/source"}
        });
        assert!(validate_status(&value).is_ok());
        let sanitized = sanitize_status(&value);
        assert_eq!(sanitized["latestReset"], json!({"id":"1","announcedAt":"2026-09-03T10:00:00+02:00","resetType":"banked","sourceType":"x_post"}));
        assert_eq!(sanitized["activeWatch"], json!({"level":"strong","chancePercent":null,"forecastWindow":"by Thursday","observedAt":"2026-09-03T09:00:00Z","expiresAt":"2026-09-04T09:00:00+00:00"}));
        assert!(!sanitized.to_string().contains("private forecast text"));
        assert!(!sanitized.to_string().contains("x.com"));
    }
    #[test]
    fn null_latest_reset_is_preserved() {
        let mut value = fixture();
        value["data"]["latest_reset"] = Value::Null;
        assert!(validate_status(&value).is_ok());
        assert_eq!(sanitize_status(&value), json!({"latestReset":null,"activeWatch":null}));
    }

    #[test]
    fn accepts_additive_api_fields_without_exposing_them() {
        let mut value = fixture();
        value["data"]["scheduled_reset"] = Value::Null;
        value["meta"]["new_optional_field"] = json!({"future": true});
        value["data"]["latest_reset"]["source"]["new_optional_field"] = json!(true);
        assert!(validate_status(&value).is_ok());
        let sanitized = sanitize_status(&value);
        assert_eq!(sanitized, json!({"latestReset":{"id":"1","announcedAt":"2026-09-03T10:00:00+02:00","resetType":"regular","sourceType":"observed"},"activeWatch":null}));
        assert!(!sanitized.to_string().contains("scheduled_reset"));
        assert!(!sanitized.to_string().contains("new_optional_field"));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "contacts the public codex-resets.com endpoint"]
    fn live_status_uses_the_same_winhttp_and_parser_as_the_application() {
        let value = ResetClient::new().fetch_status().expect("live reset status is readable");
        assert!(value.get("data").is_some());
        assert!(sanitize_status(&value).get("latestReset").is_some());
    }
}
