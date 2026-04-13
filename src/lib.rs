//! The official Rust SDK for building [Pointiv](https://pointiv.katkode.com) extensions.
//!
//! ```toml
//! [dependencies]
//! pointiv-extension-api = "0.1"
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
/// `text` and `context` are always identical — use whichever reads better.
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

// Names must match what Pointiv's wasm-host registers via extism::Function::new.
#[host_fn]
extern "ExtismHost" {
    fn pointiv_log(msg: String);

    fn pointiv_storage_read(key: String) -> String;
    fn pointiv_storage_write(key: String, value: String);
    fn pointiv_storage_delete(key: String);
    fn pointiv_storage_list() -> String;

    fn pointiv_clipboard_read() -> String;
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

pub mod prelude {
    pub use crate::{clipboard, log, storage, Input, Output};
    pub use extism_pdk::{plugin_fn, FnResult, Json};
    pub use serde::{Deserialize, Serialize};
}
