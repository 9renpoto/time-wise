#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
root=$(CDPATH='' cd -- ../.. && pwd)
channel=$(sed -n 's/^channel = "\([^"]*\)"/\1/p' "$root/rust-toolchain.toml")
RUSTC=$(rustup which --toolchain "$channel" rustc)
RUSTDOC=$(rustup which --toolchain "$channel" rustdoc)
PATH="$(dirname "$RUSTC"):$PATH"
export RUSTC RUSTDOC PATH
cargo run --quiet --locked --manifest-path "$root/apps/server/Cargo.toml" --example worker_tool -- build
