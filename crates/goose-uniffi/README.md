### goose-uniffi 

This crate is meant to create bindings so we can call goose-llm rust code from kotlin.

Docs:
- https://mozilla.github.io/uniffi-rs/latest/tutorial/foreign_language_bindings.html

Examples:
- https://github.com/mozilla/uniffi-rs/tree/main/examples
- https://github.com/mozilla/uniffi-rs/tree/main/examples/async-api-client



```
.
└── crates
    └── goose-uniffi/...
└── target
    └── debug/libgoose_uniffi.dylib
├── bindings
│   └── kotlin
│       ├── example
│       │   └── Usage.kt              ← your demo app
│       └── uniffi
│           └── goose_uniffi
│               └── goose_uniffi.kt   ← auto-generated bindings
```

Create Kotlin bindings:
```
cargo build -p goose-uniffi

cargo run --features=uniffi/cli --bin uniffi-bindgen generate --library ./target/debug/libgoose_uniffi.dylib --language kotlin --out-dir bindings/kotlin
```


From project root directory:
```
# Download JNA once (if you haven’t already)
curl -L -o jna.jar \
  https://repo1.maven.org/maven2/net/java/dev/jna/jna/5.13.0/jna-5.13.0.jar

# Compile both the generated binding and your example in a single jar:
kotlinc \
  bindings/kotlin/example/Usage.kt \
  bindings/kotlin/uniffi/goose_uniffi/goose_uniffi.kt \
  -classpath jna.jar \
  -include-runtime \
  -d example.jar

# Run it, pointing JNA at your Rust library:
java \
  -Djna.library.path=$HOME/Development/goose/target/debug \
  -cp example.jar:jna.jar \
  UsageKt
```

