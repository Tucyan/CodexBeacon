use super::snapshot;
use serde_json::{json, Map, Value};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const READ_ONLY_METHODS: [&str; 3] = [
    "account/read",
    "account/rateLimits/read",
    "account/usage/read",
];

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
const SHUTDOWN_WAIT: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexError {
    ExecutableNotFound,
    Spawn,
    Closed,
    MethodNotAllowed,
    Timeout,
    Transport,
    ProcessExit,
    Protocol,
    Rpc,
    RpcMethodNotFound,
}

impl CodexError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ExecutableNotFound => "executable_not_found",
            Self::Spawn => "spawn_error",
            Self::Closed => "closed",
            Self::MethodNotAllowed => "method_not_allowed",
            Self::Timeout => "timeout",
            Self::Transport => "transport_error",
            Self::ProcessExit => "process_exit",
            Self::Protocol => "parse_error",
            Self::Rpc => "rpc_error",
            Self::RpcMethodNotFound => "method_not_found",
        }
    }
}

/// A UTF-8 aware JSONL framer.  Bytes are retained until a complete line is
/// available, so a multibyte character split over pipe reads is not corrupted.
pub struct JsonlFramer {
    buffer: Vec<u8>,
}

impl JsonlFramer {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn push(&mut self, bytes: &[u8], out: &mut Vec<Result<Value, CodexError>>) {
        self.buffer.extend_from_slice(bytes);
        while let Some(pos) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let mut line = self.buffer.drain(..=pos).collect::<Vec<_>>();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            out.push(match serde_json::from_slice::<Value>(&line) {
                Ok(value) => Ok(value),
                Err(_) => Err(CodexError::Protocol),
            });
        }
    }

    pub fn finish(&mut self, out: &mut Vec<Result<Value, CodexError>>) {
        if !self.buffer.iter().all(u8::is_ascii_whitespace) {
            out.push(Err(CodexError::Protocol));
        }
        self.buffer.clear();
    }
}

impl Default for JsonlFramer {
    fn default() -> Self { Self::new() }
}

/// A small synchronous client around the direct vendor executable.  A reader
/// thread prevents a blocked pipe read from blocking the caller forever; only
/// the child created by this value is ever terminated.
pub struct CodexAppServerClient {
    input: Option<ChildStdin>,
    child: Child,
    messages: Receiver<Result<Value, CodexError>>,
    next_id: u64,
    timeout: Duration,
    closed: bool,
}

impl CodexAppServerClient {
    pub fn connect(path: &Path) -> Result<Self, CodexError> {
        Self::connect_with_timeout(path, DEFAULT_TIMEOUT)
    }

    pub fn connect_with_timeout(path: &Path, timeout: Duration) -> Result<Self, CodexError> {
        let mut child = Command::new(path)
            .args(["app-server", "--listen", "stdio://"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags_if_windows()
            .spawn()
            .map_err(|_| CodexError::Spawn)?;
        let input = child.stdin.take().ok_or(CodexError::Transport)?;
        let output = child.stdout.take().ok_or(CodexError::Transport)?;
        if let Some(stderr) = child.stderr.take() {
            // Drain but do not retain stderr: it can contain account or path data.
            thread::spawn(move || {
                let mut reader = BufReader::new(stderr);
                let mut ignored = [0_u8; 1024];
                loop {
                    match reader.read(&mut ignored) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {}
                    }
                }
            });
        }
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || read_pipe(output, tx));
        Ok(Self { input: Some(input), child, messages: rx, next_id: 1, timeout, closed: false })
    }

    pub fn initialize(&mut self) -> Result<Value, CodexError> {
        let result = self.request_inner(
            "initialize",
            json!({
                "clientInfo": {"name": "desktop_dashboard", "title": "Desktop Dashboard", "version": "0.1.0"},
                "capabilities": {"experimentalApi": false}
            }),
            true,
        )?;
        self.send(&json!({"method":"initialized","params":{}}))?;
        Ok(result)
    }

    pub fn request(&mut self, method: &str, params: Value) -> Result<Value, CodexError> {
        if !READ_ONLY_METHODS.contains(&method) {
            return Err(CodexError::MethodNotAllowed);
        }
        self.request_inner(method, params, false)
    }

    fn request_inner(&mut self, method: &str, params: Value, handshake: bool) -> Result<Value, CodexError> {
        if !handshake && !READ_ONLY_METHODS.contains(&method) || self.closed {
            return Err(if self.closed { CodexError::Closed } else { CodexError::MethodNotAllowed });
        }
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.send(&json!({"id": id, "method": method, "params": params}))?;
        let deadline=std::time::Instant::now()+self.timeout;
        loop {
            let remaining=deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero(){return Err(CodexError::Timeout);}
            let message = match self.messages.recv_timeout(remaining) {
                Ok(message) => message?,
                Err(RecvTimeoutError::Timeout) => return Err(CodexError::Timeout),
                Err(RecvTimeoutError::Disconnected) => {
                    self.closed = true;
                    return Err(CodexError::ProcessExit);
                }
            };
            let Some(object) = message.as_object() else { continue };
            // Id-less messages are notifications. They never satisfy a request.
            if object.get("id").is_none() { continue; }
            if object.get("id") != Some(&json!(id)) { continue; }
            if let Some(error) = object.get("error") {
                if error.get("code").and_then(Value::as_i64) == Some(-32601) {
                    return Err(CodexError::RpcMethodNotFound);
                }
                return Err(CodexError::Rpc);
            }
            return Ok(object.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    fn send(&mut self, value: &Value) -> Result<(), CodexError> {
        if self.closed { return Err(CodexError::Closed); }
        let input = self.input.as_mut().ok_or(CodexError::Closed)?;
        serde_json::to_writer(&mut *input, value).map_err(|_| CodexError::Transport)?;
        input.write_all(b"\n").map_err(|_| CodexError::Transport)?;
        input.flush().map_err(|_| CodexError::Transport)
    }

    pub fn shutdown(&mut self) {
        if self.closed && self.input.is_none() { return; }
        self.closed = true;
        self.input.take(); // EOF asks the app-server to exit cleanly.
        let deadline = std::time::Instant::now() + SHUTDOWN_WAIT;
        while std::time::Instant::now() < deadline {
            match self.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => thread::sleep(Duration::from_millis(10)),
                Err(_) => return,
            }
        }
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

impl Drop for CodexAppServerClient {
    fn drop(&mut self) { self.shutdown(); }
}

fn read_pipe<R: Read + Send + 'static>(mut output: R, tx: Sender<Result<Value, CodexError>>) {
    let mut framer = JsonlFramer::new();
    let mut bytes = [0_u8; 8192];
    loop {
        match output.read(&mut bytes) {
            Ok(0) => { let mut messages = Vec::new(); framer.finish(&mut messages); for item in messages { let _ = tx.send(item); } break; }
            Ok(count) => {
                let mut messages = Vec::new();
                framer.push(&bytes[..count], &mut messages);
                for item in messages { if tx.send(item).is_err() { return; } }
            }
            Err(_) => { let _ = tx.send(Err(CodexError::Transport)); break; }
        }
    }
}

trait CreationFlags {
    fn creation_flags_if_windows(&mut self) -> &mut Self;
}
impl CreationFlags for Command {
    fn creation_flags_if_windows(&mut self) -> &mut Self {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            self.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        self
    }
}

pub fn resolve_codex_executable(cli: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = cli {
        return path.is_file().then(|| path.to_path_buf());
    }
    if let Ok(value) = std::env::var("CODEX_EXECUTABLE") {
        let path = PathBuf::from(value);
        if path.is_file() { return Some(path); }
    }
    let appdata = std::env::var_os("APPDATA").map(PathBuf::from);
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let roots = [appdata, local];
    for root in roots.into_iter().flatten() {
        let candidate = root.join("npm/node_modules/@openai/codex/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe");
        if candidate.is_file() { return Some(candidate); }
    }
    None
}

pub fn sanitize_account(value: Option<&Value>) -> Value {
    let source = value.and_then(|v| v.get("account")).and_then(Value::as_object).or_else(|| value.and_then(Value::as_object));
    json!({
        "authType": source.and_then(|v| v.get("type")).and_then(Value::as_str),
        "planType": source.and_then(|v| v.get("planType")).and_then(Value::as_str),
    })
}

fn safe_number(value: Option<&Value>) -> Option<Value> {
    let value = value?;
    if value.as_i64().is_some() || value.as_u64().is_some() || value.as_f64().is_some_and(|number| number.is_finite()) { Some(value.clone()) } else { None }
}

fn sanitize_window(value: Option<&Value>) -> Value {
    let Some(source) = value.and_then(Value::as_object) else { return Value::Null };
    let mut out = Map::new();
    for key in ["usedPercent", "windowDurationMins", "resetsAt"] {
        if let Some(number) = safe_number(source.get(key)) { out.insert(key.to_owned(), number); }
    }
    Value::Object(out)
}

fn sanitize_bucket(id: Option<&str>, value: Option<&Value>) -> Value {
    let source = value.and_then(Value::as_object);
    let mut out = Map::new();
    out.insert("limitId".to_owned(), id.map_or(Value::Null, |v| Value::String(v.to_owned())));
    out.insert("primary".to_owned(), sanitize_window(source.and_then(|v| v.get("primary"))));
    out.insert("secondary".to_owned(), sanitize_window(source.and_then(|v| v.get("secondary"))));
    Value::Object(out)
}

pub fn sanitize_rate_limits(value: Option<&Value>) -> Value {
    let Some(source) = value.and_then(Value::as_object) else { return Value::Null };
    let mut buckets = Vec::new();
    if let Some(map) = source.get("rateLimitsByLimitId").and_then(Value::as_object) {
        for (id, item) in map { buckets.push(sanitize_bucket(item.get("limitId").and_then(Value::as_str).or(Some(id)), Some(item))); }
    } else {
        let primary = source.get("rateLimits").filter(|v| v.is_object()).unwrap_or(value.unwrap());
        buckets.push(sanitize_bucket(primary.get("limitId").and_then(Value::as_str), Some(primary)));
    }
    let mut out = Map::new();
    out.insert("buckets".into(), Value::Array(buckets));
    if let Some(number) = safe_number(source.get("rateLimitResetCredits").and_then(|v| v.get("availableCount"))) { out.insert("resetCreditsAvailableCount".into(), number); }
    Value::Object(out)
}

pub fn sanitize_usage(value: Option<&Value>) -> Value {
    let Some(summary) = value.and_then(|v| v.get("summary")).and_then(Value::as_object) else { return Value::Null };
    let mut out = Map::new();
    for key in ["lifetimeTokens", "peakDailyTokens", "longestRunningTurnSec", "currentStreakDays", "longestStreakDays"] {
        if let Some(number) = safe_number(summary.get(key)) { out.insert(key.to_owned(), number); }
    }
    json!({"summary": out})
}

pub fn sanitize_snapshot(account: Option<&Value>, limits: Option<&Value>, usage: Option<&Value>) -> Value {
    json!({"account": sanitize_account(account), "rateLimits": sanitize_rate_limits(limits), "usage": sanitize_usage(usage)})
}

fn now_seconds() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|duration| duration.as_secs_f64()).unwrap_or(0.0)
}

pub fn read_codex(cli: Option<&Path>) -> crate::model::ProviderSnapshot {
    let Some(path) = resolve_codex_executable(cli) else {
        return snapshot("unavailable", None, Some("executable_not_found"), None);
    };
    let mut client = match CodexAppServerClient::connect(&path) {
        Ok(client) => client,
        Err(error) => return snapshot("offline", None, Some(error.code()), None),
    };
    let result = (|| {
        client.initialize()?;
        let account = client.request("account/read", json!({"refreshToken": false}))?;
        let limits = client.request("account/rateLimits/read", Value::Null)?;
        let usage = client.request("account/usage/read", Value::Null)?;
        Ok::<_, CodexError>(sanitize_snapshot(Some(&account), Some(&limits), Some(&usage)))
    })();
    client.shutdown();
    match result {
        Ok(data) => snapshot("ready", Some(now_seconds()), None, Some(data)),
        Err(error) => snapshot("error", None, Some(error.code()), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn framer_handles_split_json_and_utf8() {
        let mut framer = JsonlFramer::new();
        let mut out = Vec::new();
        let mut bytes = serde_json::to_vec(&json!({"params":{"label":"额度"}})).unwrap();
        bytes.push(b'\n');
        framer.push(&bytes[..bytes.len() - 2], &mut out);
        framer.push(&bytes[bytes.len() - 2..], &mut out);
        assert_eq!(out.len(), 1);
        assert!(out[0].is_ok());
    }
    #[test]
    fn sanitizer_drops_secrets_and_unknowns() {
        let account = json!({"account":{"type":"chatgpt","planType":"plus","email":"secret@example.com"},"requiresOpenaiAuth":true});
        let limits = json!({"rateLimitsByLimitId":{"standard":{"primary":{"usedPercent":12,"resetsAt":123,"email":"x"}}},"rateLimitResetCredits":{"availableCount":1}});
        let usage = json!({"summary":{"lifetimeTokens":4,"email":"x","currentStreakDays":2}});
        let value = sanitize_snapshot(Some(&account), Some(&limits), Some(&usage));
        assert!(value.to_string().contains("chatgpt"));
        assert!(!value.to_string().contains("secret@example.com"));
        assert!(!value.to_string().contains("email"));
    }
}
