#!/bin/sh
# Bootstrap only the repository-pinned compiler and Wasm target for Workers Builds.
set -eu
cd "$(dirname "$0")/../../.."
PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
export PATH
if ! command -v rustup >/dev/null 2>&1; then
    installer=$(mktemp)
    trap 'rm -f "$installer"' EXIT HUP INT TERM
    curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs -o "$installer"
    sh "$installer" -y --profile minimal --default-toolchain none --no-modify-path
    rm -f "$installer"
    trap - EXIT HUP INT TERM
fi
channel=$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)
[ -n "$channel" ] || { echo 'Missing pinned Rust channel' >&2; exit 1; }
if ! rustup which --toolchain "$channel" rustc >/dev/null 2>&1; then
    rustup toolchain install "$channel" --profile minimal --no-self-update
fi
rustup target add --toolchain "$channel" wasm32-unknown-unknown
RUSTC=$(rustup which --toolchain "$channel" rustc)
RUSTDOC=$(rustup which --toolchain "$channel" rustdoc)
PATH="$(dirname "$RUSTC"):$PATH"
export RUSTC RUSTDOC PATH
exec "$@"
