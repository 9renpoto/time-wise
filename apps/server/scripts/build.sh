#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
root=$(CDPATH='' cd -- ../.. && pwd)
channel=$(sed -n 's/^channel = "\([^"]*\)"/\1/p' "$root/rust-toolchain.toml")
RUSTC=$(rustup which --toolchain "$channel" rustc)
RUSTDOC=$(rustup which --toolchain "$channel" rustdoc)
PATH="$(dirname "$RUSTC"):$PATH"
export RUSTC RUSTDOC PATH
builder=${WORKER_BUILD:-worker-build}
if [ "$("$builder" --version)" != "0.8.7" ]; then
    echo 'Install worker-build 0.8.7 with cargo install worker-build --version 0.8.7 --locked' >&2
    exit 1
fi
"$builder" --release --locked
