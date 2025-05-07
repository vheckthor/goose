### goose-uniffi 

This crate is meant to create bindings so we can call goose-llm rust code from kotlin.

Docs:
- https://mozilla.github.io/uniffi-rs/latest/tutorial/foreign_language_bindings.html

Examples:
- https://github.com/mozilla/uniffi-rs/tree/main/examples
- https://github.com/mozilla/uniffi-rs/tree/main/examples/async-api-client


Run:
```
cargo run --features=uniffi/cli --bin uniffi-bindgen generate --library ./target/debug/libgoose_uniffi.dylib --language kotlin --out-dir out
```

