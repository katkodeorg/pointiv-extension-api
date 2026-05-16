# pointiv-extension-api

Rust SDK for [Pointiv](https://pointiv.katkode.com) WASM extensions.

## Setup

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
pointiv-extension-api = "0.2.1"
extism-pdk = "1"
```

```rust
use pointiv_extension_api::prelude::*;

#[plugin_fn]
pub fn execute(Json(input): Json<Input>) -> FnResult<Json<Output>> {
    Ok(Json(Output::text(format!("Hello, {}!", input.text))))
}
```

Build:

```sh
cargo build --release --target wasm32-wasip1
```

## Input and output

`Input` has `text`, `context` (same as `text`), and `command` (what the user typed in the popup).

`Output` helpers: `text`, `copy`, `type_text`, `error`.

## APIs

Add permissions in `pointiv-extension.json`. Without a permission, calls fail safely (empty string, status 403, or an error JSON field).

| Module | Permission | What it does |
|--------|------------|--------------|
| `storage::` | `storage` | Per-extension key/value store |
| `clipboard::` | `clipboard_read` | Read clipboard |
| `ai::` | `ai` | LLM completion |
| `http::` | `network` | Outbound HTTP (you supply auth headers) |
| `google_calendar::` | `google_calendar` | Create Calendar events (Pointiv injects JWT) |
| `google_gmail::` | `google_gmail` | Send Gmail (Pointiv injects JWT) |
| `log::` | none | Log to `~/.pointiv/trace.jsonl` |

### Storage

```rust
storage::write("key", "value");
let v = storage::read("key");
storage::write_json("key", &my_struct);
let s: Option<MyStruct> = storage::read_json("key");
```

### HTTP

```rust
let resp = http::get("https://api.example.com/data");
let resp = http::post("https://api.example.com", r#"{"x":1}"#);

let resp = http::request(HttpRequest {
    method: "GET".into(),
    url: "https://api.example.com".into(),
    headers: [("Authorization".into(), "Bearer token".into())].into(),
    body: String::new(),
});
```

### Google Calendar

Connect Google in Pointiv Settings first.

```rust
google_calendar::schedule(
    "Team standup",
    "2026-06-01",
    Some("09:00"),
    Some("09:30"),
    Some("Daily sync"),
)?;
```

### Gmail

```rust
google_gmail::send("you@example.com", "Subject", "Body text")?;
```

### Logging

```rust
log::info("started");
log::warn("slow response");
log::error("failed");
```

## Manifest

`pointiv-extension.json` at the repo root:

```json
{
  "id": "community.your-name.my-extension",
  "name": "My Extension",
  "description": "What it does",
  "version": "1.0.0",
  "author": "your-name",
  "keywords": ["tag"],
  "runtime": "wasm",
  "main": "extension.wasm",
  "permissions": ["storage", "network", "google_calendar", "google_gmail"]
}
```

## License

MIT
