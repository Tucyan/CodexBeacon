//! Bounded provider workers.  The root owns the returned JoinHandles and sets
//! `AppState::stopping` before joining them during application shutdown.

use super::{codex, reset, snapshot};
use crate::AppState;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const POLL_QUANTUM: Duration = Duration::from_millis(100);
const CODEX_LIMITS_PERIOD: Duration = Duration::from_secs(60);
const CODEX_USAGE_PERIOD: Duration = Duration::from_secs(300);
const RESET_PERIOD: Duration = Duration::from_secs(900);
const INITIAL_BACKOFF: Duration = Duration::from_secs(5);
const MAX_BACKOFF: Duration = Duration::from_secs(60);

/// Start the two independent provider loops. No network or child process is
/// started by this function itself; each loop performs its first read on its
/// worker thread, keeping the UI caller bounded.
pub fn spawn(app: AppHandle) -> Vec<JoinHandle<()>> {
    let codex_app = app.clone();
    let reset_app = app;
    vec![
        thread::spawn(move || run_codex(codex_app)),
        thread::spawn(move || run_reset(reset_app)),
    ]
}

fn stopped(app: &AppHandle) -> bool {
    app.state::<AppState>().stopping.load(Ordering::Acquire)
}

fn take_retry(app: &AppHandle, codex_provider: bool) -> bool {
    let state = app.state::<AppState>();
    if codex_provider {
        state.retry_codex.swap(false, Ordering::AcqRel)
    } else {
        state.retry_reset.swap(false, Ordering::AcqRel)
    }
}

/// Wait in at most 100ms pieces. The retry flag is a wakeup, not a second
/// request path; callers decide whether a provider's backoff still applies.
enum Wake {
    Stopped,
    Deadline,
    Retry,
}

fn wait_until(app: &AppHandle, deadline: Instant, codex_provider: bool) -> Wake {
    loop {
        if stopped(app) {
            return Wake::Stopped;
        }
        if take_retry(app, codex_provider) {
            return Wake::Retry;
        }
        if Instant::now() >= deadline {
            return Wake::Deadline;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        thread::sleep(remaining.min(POLL_QUANTUM));
    }
}

fn unix_seconds() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|value| value.as_secs_f64()).unwrap_or(0.0)
}

fn publish(app: &AppHandle, name: &str, state: &str, last_success: Option<f64>, error: Option<&str>, data: Option<Value>) {
    crate::publish_provider(app, name, snapshot(state, last_success, error, data));
}

fn safe_codex_data(account: Option<&Value>, limits: Option<&Value>, usage: Option<&Value>) -> Value {
    codex::sanitize_snapshot(account, limits, usage)
}

fn run_codex(app: AppHandle) {
    let mut client: Option<codex::CodexAppServerClient> = None;
    let mut account: Option<Value> = None;
    let mut limits: Option<Value> = None;
    let mut usage: Option<Value> = None;
    let mut last_success = None;
    let mut next_connect = Instant::now();
    let mut next_limits = Instant::now();
    let mut next_usage = Instant::now();
    let mut backoff = INITIAL_BACKOFF;

    publish(&app, "codex", "loading", None, None, None);
    while !stopped(&app) {
        let forced = take_retry(&app, true);
        if forced {
            next_connect = Instant::now();
            next_limits = Instant::now();
            next_usage = Instant::now();
        }

        if client.is_none() {
            if matches!(wait_until(&app, next_connect, true), Wake::Stopped) { break; }
            let Some(executable) = codex::resolve_codex_executable(None) else {
                publish(&app, "codex", "unavailable", last_success, Some("executable_not_found"), None);
                next_connect = Instant::now() + backoff;
                backoff = (backoff * 2).min(MAX_BACKOFF);
                continue;
            };
            publish(&app, "codex", "loading", last_success, None, None);
            match codex::CodexAppServerClient::connect(Path::new(&executable)).and_then(|mut value| {
                value.initialize().map(|_| value)
            }) {
                Ok(value) => {
                    client = Some(value);
                    account = None;
                    limits = None;
                    usage = None;
                    next_limits = Instant::now();
                    next_usage = Instant::now();
                    backoff = INITIAL_BACKOFF;
                }
                Err(error) => {
                    publish(&app, "codex", "offline", last_success, Some(error.code()), None);
                    next_connect = Instant::now() + backoff;
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                    continue;
                }
            }
        }

        let now = Instant::now();
        let limits_due = now >= next_limits;
        let usage_due = now >= next_usage;
        if !limits_due && !usage_due {
            match wait_until(&app, next_limits.min(next_usage), true) {
                Wake::Stopped => break,
                Wake::Retry => {
                    next_limits = Instant::now();
                    next_usage = Instant::now();
                }
                Wake::Deadline => {}
            }
            continue;
        }

        let request_result = (|| {
            let value = client.as_mut().ok_or(codex::CodexError::Closed)?;
            if limits_due {
                account = Some(value.request("account/read", json!({"refreshToken": false}))?);
                limits = Some(value.request("account/rateLimits/read", Value::Null)?);
                next_limits = Instant::now() + CODEX_LIMITS_PERIOD;
            }
            if usage_due {
                match value.request("account/usage/read", Value::Null) {
                    Ok(value) => usage = Some(value),
                    Err(codex::CodexError::RpcMethodNotFound) => usage = None,
                    Err(error) => return Err(error),
                }
                next_usage = Instant::now() + CODEX_USAGE_PERIOD;
            }
            Ok::<(), codex::CodexError>(())
        })();

        match request_result {
            Ok(()) => {
                last_success = Some(unix_seconds());
                publish(&app, "codex", "ready", last_success, None, Some(safe_codex_data(account.as_ref(), limits.as_ref(), usage.as_ref())));
            }
            Err(error) => {
                if let Some(value) = client.as_mut() { value.shutdown(); }
                client = None;
                publish(&app, "codex", "error", last_success, Some(error.code()), Some(safe_codex_data(account.as_ref(), limits.as_ref(), usage.as_ref())));
                next_connect = Instant::now() + backoff;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
        }
    }
    if let Some(mut value) = client { value.shutdown(); }
}

fn run_reset(app: AppHandle) {
    let mut client = reset::ResetClient::new();
    let mut next_fetch = Instant::now();
    let mut retry_not_before = Instant::now();
    let mut last_success = None;

    publish(&app, "reset", "loading", None, None, None);
    while !stopped(&app) {
        let forced = take_retry(&app, false);
        if forced {
            if Instant::now() >= retry_not_before {
                next_fetch = Instant::now();
            }
            // A retry before Retry-After is consumed and intentionally does
            // not re-arm itself; the normal deadline remains authoritative.
        }
        let deadline = next_fetch.max(retry_not_before);
        if matches!(wait_until(&app, deadline, false), Wake::Stopped) { break; }
        if Instant::now() < retry_not_before { continue; }

        publish(&app, "reset", "loading", last_success, None, None);
        match client.fetch_status() {
            Ok(value) => {
                last_success = Some(unix_seconds());
                retry_not_before = Instant::now();
                next_fetch = Instant::now() + RESET_PERIOD;
                publish(&app, "reset", "ready", last_success, None, Some(reset::sanitize_status(&value)));
            }
            Err(error) => {
                let retry_after = client.retry_after_ms().map(Duration::from_millis).unwrap_or_default();
                retry_not_before = Instant::now() + retry_after;
                next_fetch = Instant::now() + RESET_PERIOD;
                publish(&app, "reset", "error", last_success, Some(error.code()), None);
            }
        }
    }
}
