#!/bin/bash

cargo build --release && cargo build --release --target wasm32-unknown-unknown && wasm-bindgen --out-dir ./docs/ --target no-modules ./target/wasm32-unknown-unknown/release/rusty-boids.wasm && cp -R ./assets ./target/release/assets
