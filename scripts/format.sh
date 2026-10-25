#!/usr/bin/env sh
# Format and lint the workspace.
set -e
cargo fmt
cargo clippy --workspace --all-targets --all-features
