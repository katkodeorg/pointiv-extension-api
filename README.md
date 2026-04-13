# pointiv-extension-api

Rust SDK for building [Pointiv](https://pointiv.katkode.com) extensions.

## Getting started

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
pointiv-extension-api = "0.1"
extism-pdk = "1"
```

```rust
use pointiv_extension_api::prelude::*;

#[plugin_fn]
pub fn execute(Json(input): Json<Input>) -> FnResult<Json<Output>> {
    let count: u64 = storage::read("runs")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0) + 1;
    storage::write("runs", &count.to_string());

    Ok(Json(Output::text(format!("Hello, {}! (run #{})", input.text, count))))
}
```

Build with:

```sh
cargo build --release --target wasm32-wasip1
```

## Input

| Field     | Description                                    |
|-----------|------------------------------------------------|
| `text`    | Selected text or clipboard contents            |
| `context` | Same as `text`                                 |
| `command` | What the user typed in the Pointiv popup       |

## Output

| Method                 | Effect                                      |
|------------------------|---------------------------------------------|
| `Output::text(v)`      | Show in result panel                        |
| `Output::copy(v)`      | Copy to clipboard                           |
| `Output::type_text(v)` | Type into the previously focused window     |
| `Output::error(v)`     | Show as an error                            |

## APIs

**`storage::`** — sandboxed per-extension key/value store. Requires `"storage"` permission.

```rust
storage::write("key", "value");
let v = storage::read("key");           // Option<String>
storage::write_json("key", &my_struct);
let s: Option<MyStruct> = storage::read_json("key");
```

**`log::`** — writes to `~/.pointiv/trace.jsonl`. No permission needed.

```rust
log::info("msg");
log::warn("msg");
log::error("msg");
```

**`clipboard::`** — requires `"clipboard_read"` permission.

```rust
let text = clipboard::read();
```

## Manifest

Every extension needs a `pointiv-extension.json` at the repo root:

```json
{
  "id":          "your-org.extension-name",
  "name":        "My Extension",
  "description": "What it does",
  "version":     "1.0.0",
  "author":      "your-org",
  "keywords":    ["word1", "word2"],
  "runtime":     "wasm",
  "main":        "extension.wasm",
  "permissions": ["storage"]
}
```

## License

MIT
