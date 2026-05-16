//! The official Rust SDK for building [Pointiv](https://pointiv.katkode.com) extensions.
//!
//! ```toml
//! [dependencies]
//! pointiv-extension-api = "0.2.1"
//! extism-pdk = "1"
//! ```
//!
//! ```rust,no_run
//! use pointiv_extension_api::prelude::*;
//!
//! #[plugin_fn]
//! pub fn execute(Json(input): Json<Input>) -> FnResult<Json<Output>> {
//!     Ok(Json(Output::text(format!("Hello, {}!", input.text))))
//! }
//! ```

pub use extism_pdk::{host_fn, plugin_fn, FnResult, Json};
pub use serde::{Deserialize, Serialize};

/// Input passed to your `execute` function by Pointiv.
///
/// `text` and `context` are the same value. Use either field.
#[derive(Debug, Clone, Deserialize)]
pub struct Input {
    /// Selected text or clipboard contents. Empty if nothing was captured.
    pub text: String,
    /// Alias for `text`.
    pub context: String,
    /// The command the user typed in the popup.
    pub command: String,
}

/// What your extension returns.
///
/// ```rust,no_run
/// # use pointiv_extension_api::Output;
/// Output::text("result")         // display in result panel
/// Output::copy("value")          // copy to clipboard
/// Output::type_text("value")     // type into the previously focused window
/// Output::error("oops")          // show as an error
/// # ;
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct Output {
    #[serde(rename = "type")]
    pub kind: String,
    pub value: String,
}

impl Output {
    pub fn text(value: impl Into<String>) -> Self {
        Self { kind: "text".into(), value: value.into() }
    }

    pub fn copy(value: impl Into<String>) -> Self {
        Self { kind: "copy".into(), value: value.into() }
    }

    pub fn type_text(value: impl Into<String>) -> Self {
        Self { kind: "type".into(), value: value.into() }
    }

    pub fn error(value: impl Into<String>) -> Self {
        Self { kind: "error".into(), value: value.into() }
    }
}

/// Outbound HTTP request passed from a WASM extension to `pointiv_http_request`.
///
/// Serialize with `serde_json::to_string` and pass to [`http::request`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
}

/// Response returned by `pointiv_http_request` to the WASM extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

// Names must match what Pointiv's wasm-host registers via extism::Function::new.
#[host_fn]
extern "ExtismHost" {
    fn pointiv_log(msg: String);

    fn pointiv_storage_read(key: String) -> String;
    fn pointiv_storage_write(key: String, value: String);
    fn pointiv_storage_delete(key: String);
    fn pointiv_storage_list() -> String;

    fn pointiv_clipboard_read() -> String;

    /// Send a prompt to the Pointiv LLM and return the plain-text reply.
    /// Returns an empty string if the extension does not have the `"ai"` permission.
    fn pointiv_ai_complete(prompt: String) -> String;

    /// Make an outbound HTTP request. Requires `"network"` in your manifest permissions.
    /// Returns `HttpResponse { status: 403, body: "" }` if the permission is not granted.
    /// Argument: serialized [`HttpRequest`] JSON.
    fn pointiv_http_request(request_json: String) -> String;

    /// Create a calendar event via the backend proxy. Requires `"google_calendar"`.
    /// Host injects the Pointiv JWT. Argument: event JSON. Returns Google API response JSON.
    fn pointiv_google_calendar_create(payload_json: String) -> String;

    /// Send Gmail via the Pointiv backend proxy. Requires `"google_gmail"` permission.
    /// Argument JSON: `{ "to", "subject", "body" }`.
    fn pointiv_google_gmail_send(payload_json: String) -> String;
}

/// Structured logging to `~/.pointiv/trace.jsonl`. No permission required.
pub mod log {
    use super::pointiv_log;

    pub fn info(msg: &str)  { emit("INFO",  msg); }
    pub fn warn(msg: &str)  { emit("WARN",  msg); }
    pub fn error(msg: &str) { emit("ERROR", msg); }

    fn emit(level: &str, msg: &str) {
        let _ = unsafe { pointiv_log(format!("[{level}] {msg}")) };
    }
}

/// Sandboxed key/value storage isolated per extension.
///
/// Requires `"storage"` in your `pointiv-extension.json` permissions.
/// Calls without the permission are safe and silently return empty/None.
pub mod storage {
    use super::{
        pointiv_storage_delete, pointiv_storage_list, pointiv_storage_read, pointiv_storage_write,
    };
    use serde::{Serialize, de::DeserializeOwned};

    pub fn write(key: &str, value: &str) {
        let _ = unsafe { pointiv_storage_write(key.to_string(), value.to_string()) };
    }

    pub fn read(key: &str) -> Option<String> {
        let s = unsafe { pointiv_storage_read(key.to_string()).ok()? };
        if s.is_empty() { None } else { Some(s) }
    }

    pub fn delete(key: &str) {
        let _ = unsafe { pointiv_storage_delete(key.to_string()) };
    }

    pub fn list() -> Vec<String> {
        let json = unsafe { pointiv_storage_list().ok() }.unwrap_or_default();
        serde_json::from_str::<Vec<String>>(&json).unwrap_or_default()
    }

    pub fn read_json<T: DeserializeOwned>(key: &str) -> Option<T> {
        read(key).and_then(|s| serde_json::from_str(&s).ok())
    }

    pub fn write_json<T: Serialize>(key: &str, value: &T) {
        if let Ok(json) = serde_json::to_string(value) {
            write(key, &json);
        }
    }
}

/// Read the system clipboard. Requires `"clipboard_read"` permission.
pub mod clipboard {
    use super::pointiv_clipboard_read;

    pub fn read() -> String {
        unsafe { pointiv_clipboard_read().ok() }.unwrap_or_default()
    }
}

/// Call the Pointiv LLM. Requires `"ai"` in your `pointiv-extension.json` permissions.
///
/// Send a prompt, get plain text back. No tool chaining.
///
/// ```rust,no_run
/// use pointiv_extension_api::prelude::*;
///
/// #[plugin_fn]
/// pub fn execute(Json(input): Json<Input>) -> FnResult<Json<Output>> {
///     let summary = ai::complete(&format!(
///         "Summarise this in one sentence: {}", input.text
///     ));
///     Ok(Json(Output::text(summary)))
/// }
/// ```
pub mod ai {
    use super::pointiv_ai_complete;

    /// Send `prompt` to the LLM and return the plain-text completion.
    /// Returns an empty string if the `"ai"` permission was not granted.
    pub fn complete(prompt: &str) -> String {
        unsafe { pointiv_ai_complete(prompt.to_string()).ok() }.unwrap_or_default()
    }
}

/// Make outbound HTTP requests. Requires `"network"` in `pointiv-extension.json` permissions.
///
/// Raw HTTP. Put your own auth in request headers. Pointiv does not add credentials.
///
/// ```rust,no_run
/// use pointiv_extension_api::prelude::*;
///
/// #[plugin_fn]
/// pub fn execute(Json(input): Json<Input>) -> FnResult<Json<Output>> {
///     let resp = http::get("https://api.example.com/data");
///     Ok(Json(Output::text(resp.body)))
/// }
/// ```
pub mod http {
    use super::{pointiv_http_request, HttpRequest, HttpResponse};
    use std::collections::HashMap;

    /// Make a fully custom HTTP request.
    /// Returns a fallback `{ status: 0 }` response if the call fails internally.
    pub fn request(req: HttpRequest) -> HttpResponse {
        let json = serde_json::to_string(&req).unwrap_or_default();
        let raw = unsafe { pointiv_http_request(json).ok() }.unwrap_or_default();
        serde_json::from_str::<HttpResponse>(&raw).unwrap_or(HttpResponse {
            status: 0,
            body: String::new(),
        })
    }

    /// Convenience: GET request with no extra headers or body.
    pub fn get(url: &str) -> HttpResponse {
        request(HttpRequest {
            method: "GET".into(),
            url: url.to_string(),
            headers: HashMap::new(),
            body: String::new(),
        })
    }

    /// Convenience: POST request with a plain-text body and no extra headers.
    pub fn post(url: &str, body: &str) -> HttpResponse {
        request(HttpRequest {
            method: "POST".into(),
            url: url.to_string(),
            headers: HashMap::new(),
            body: body.to_string(),
        })
    }
}

/// Create Google Calendar events via the Pointiv backend proxy.
/// Requires `"google_calendar"` in `pointiv-extension.json` permissions.
/// Host injects the Pointiv JWT. Your extension never sees Google credentials.
pub mod google_calendar {
    use super::pointiv_google_calendar_create;

    /// Create an event. Prefer this over [`create_event_raw`].
    ///
    /// `date` is `YYYY-MM-DD`. `start_time` and `end_time` are `HH:MM` (24h), or `None`.
    /// All-day if `start_time` is `None`. Default end is start + 1 hour when `end_time` is `None`.
    /// Returns `Ok(response_json)` or `Err(message)`.
    ///
    /// ```rust,no_run
    /// use pointiv_extension_api::prelude::*;
    ///
    /// google_calendar::schedule(
    ///     "Team standup",
    ///     "2026-06-01",
    ///     Some("09:00"),
    ///     Some("09:30"),
    ///     Some("Daily sync"),
    /// ).unwrap();
    /// ```
    pub fn schedule(
        title: &str,
        date: &str,          // "YYYY-MM-DD"
        start_time: Option<&str>, // "HH:MM" or None → all-day
        end_time: Option<&str>,   // "HH:MM" or None → start + 1h
        description: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let (start_obj, end_obj) = match start_time {
            None => {
                // All-day: end date is the next calendar day.
                let end_date = next_day(date).unwrap_or_else(|| date.to_string());
                (
                    serde_json::json!({ "date": date }),
                    serde_json::json!({ "date": end_date }),
                )
            }
            Some(s) => {
                let end = end_time.unwrap_or_else(|| "");
                let end_hm = if end.is_empty() { add_one_hour(s) } else { end.to_string() };
                (
                    serde_json::json!({ "dateTime": format!("{date}T{s}:00"), "timeZone": "UTC" }),
                    serde_json::json!({ "dateTime": format!("{date}T{end_hm}:00"), "timeZone": "UTC" }),
                )
            }
        };

        let payload = serde_json::json!({
            "summary":     title,
            "description": description.unwrap_or(""),
            "start":       start_obj,
            "end":         end_obj,
        });
        create_event_raw(&payload)
    }

    /// Pass a raw Google Calendar event object. Use [`schedule`] for the common case.
    pub fn create_event_raw(payload: &serde_json::Value) -> Result<serde_json::Value, String> {
        let json = serde_json::to_string(payload).map_err(|e| e.to_string())?;
        let raw = unsafe { pointiv_google_calendar_create(json).ok() }
            .unwrap_or_else(|| r#"{"error":"host call failed"}"#.to_string());
        let result: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        if let Some(err) = result["error"].as_str() {
            return Err(err.to_string());
        }
        Ok(result)
    }

    fn next_day(date: &str) -> Option<String> {
        // Handles month and year rollover for all-day end dates.
        let parts: Vec<&str> = date.split('-').collect();
        if parts.len() != 3 { return None; }
        let y: u32 = parts[0].parse().ok()?;
        let m: u32 = parts[1].parse().ok()?;
        let d: u32 = parts[2].parse().ok()?;
        let days_in_month = match m {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) => 29,
            2 => 28,
            _ => return None,
        };
        let (ny, nm, nd) = if d < days_in_month {
            (y, m, d + 1)
        } else if m < 12 {
            (y, m + 1, 1)
        } else {
            (y + 1, 1, 1)
        };
        Some(format!("{ny:04}-{nm:02}-{nd:02}"))
    }

    fn add_one_hour(hm: &str) -> String {
        let parts: Vec<&str> = hm.split(':').collect();
        if parts.len() != 2 { return hm.to_string(); }
        let h: u32 = parts[0].parse().unwrap_or(0);
        let m: u32 = parts[1].parse().unwrap_or(0);
        format!("{:02}:{:02}", (h + 1) % 24, m)
    }
}

/// Send Gmail via the Pointiv backend proxy.
/// Requires `"google_gmail"` in `pointiv-extension.json` permissions.
/// Host injects the Pointiv JWT, same as [`google_calendar`].
pub mod google_gmail {
    use super::pointiv_google_gmail_send;

    /// Send an email via Gmail. Returns `Ok(response_json)` or `Err(error_message)`.
    /// Returns an error if the `"google_gmail"` permission was not granted.
    pub fn send(to: &str, subject: &str, body: &str) -> Result<serde_json::Value, String> {
        let payload = serde_json::json!({ "to": to, "subject": subject, "body": body });
        let json = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
        let raw = unsafe { pointiv_google_gmail_send(json).ok() }
            .unwrap_or_else(|| r#"{"error":"host call failed"}"#.to_string());
        let result: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        if let Some(err) = result["error"].as_str() {
            return Err(err.to_string());
        }
        Ok(result)
    }
}

pub mod prelude {
    pub use crate::{
        ai, clipboard, google_calendar, google_gmail, http, log, storage,
        HttpRequest, HttpResponse, Input, Output,
    };
    pub use extism_pdk::{plugin_fn, FnResult, Json};
    pub use serde::{Deserialize, Serialize};
}
