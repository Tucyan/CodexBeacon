# Read-only providers

This module implements the provider boundary from `docs/development/CONTRACT.md`.

`codex.rs` starts only the direct vendor `codex.exe` selected by the optional
path, `CODEX_EXECUTABLE`, or the installed npm vendor location. It speaks the
stdio JSONL app-server protocol, sends `initialize` with
`experimentalApi: false`, then `initialized`, `account/read` with
`refreshToken: false`, `account/rateLimits/read`, and `account/usage/read`.
Only those three account methods are accepted after the handshake. Responses
and notifications are separated by JSON-RPC id; notifications are ignored.
The child stdout has a bounded request timeout, stderr is drained and never
stored, and shutdown closes and then terminates only the owned child after a
bounded grace period.

The sanitizer exposes only `account.authType/planType`, quota bucket windows
and `resetCreditsAvailableCount`, and the five usage summary aggregates. It
does not expose raw response fields, account identifiers, or RPC error text.

`reset.rs` reads the fixed public `GET https://codex-resets.com/api/v1/status`
endpoint using Windows WinHTTP (`windows-sys` feature
`Win32_Networking_WinHttp`). Requests use five second WinHTTP timeouts, GET
only, no credentials, no retries, and conditional `If-None-Match` requests.
Validated 304 responses use the client's in-memory cache. A 429 is surfaced as
`rate_limited`; `retry_after_ms()` is available to the root scheduler and the
provider never sleeps or retries itself. The public data shape contains only
`latestReset.id`, `announcedAt`, and `resetType`, with a null latest reset
preserved.

Pure tests cover split UTF-8 JSONL framing, notification-safe sanitization,
strict reset envelope validation, source validation, and nullable reset data.
No Cargo command, network request, or real Codex executable was run while
writing this module. Root should expose `pub mod providers;` and schedule a
persistent `ResetClient` if ETag reuse is desired.
