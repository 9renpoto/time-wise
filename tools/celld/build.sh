#!/bin/sh
set -eu

directory=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH='' cd -- "$directory/../.." && pwd)
toolchain=$(sed -n 's/^channel = "\([^"]*\)"/\1/p' "$root/rust-toolchain.toml")

mkdir -p "$directory/fixture/build"
rustup run "$toolchain" rustc \
    --edition 2021 \
    --crate-type cdylib \
    --target wasm32-unknown-unknown \
    -C opt-level=s \
    -C panic=abort \
    -o "$directory/fixture/build/probe.wasm" \
    "$directory/probe.rs"
