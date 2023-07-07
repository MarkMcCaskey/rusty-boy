#!/bin/bash
cargo build --release --target=wasm32-unknown-unknown --no-default-features --lib
wasm-opt -O target/wasm32-unknown-unknown/release/rusty_boy_lib.wasm -o target/wasm32-unknown-unknown/release/opt.wasm
wasm-strip target/wasm32-unknown-unknown/release/opt.wasm -o target/wasm32-unknown-unknown/release/opt.wasm
