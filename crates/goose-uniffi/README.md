### goose-uniffi 

This crate is meant to create bindings so we can call goose-llm rust code from kotlin.

Docs:
- https://mozilla.github.io/uniffi-rs/latest/tutorial/foreign_language_bindings.html

Examples:
- https://github.com/mozilla/uniffi-rs/tree/main/examples
- https://github.com/mozilla/uniffi-rs/tree/main/examples/async-api-client


Create Kotlin bindings:
```
cargo run --features=uniffi/cli --bin uniffi-bindgen generate --library ./target/debug/libgoose_uniffi.dylib --language kotlin --out-dir out
```

Run Example:
```
cd out/uniffi/goose_uniffi

# Download JNA
curl -L -o jna.jar \
  https://repo1.maven.org/maven2/net/java/dev/jna/jna/5.13.0/jna-5.13.0.jar

# Compile kotlin to usage.jar
kotlinc usage.kt goose_uniffi.kt \
  -classpath jna.jar \
  -include-runtime \
  -d usage.jar

# Run the program
java \
  -Djna.library.path=$HOME/Development/goose/target/debug \
  -cp usage.jar:jna.jar \
  UsageKt
```