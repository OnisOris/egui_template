#!/usr/bin/env bash
set -euo pipefail

python3 scripts/check-rust-lines.py
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
duplicates="$(cargo tree -d)"
printf '%s\n' "$duplicates"
egui_versions="$(cargo tree --workspace --prefix none | awk '$1 == "egui" { print $2 }' | sort -u)"
if [[ "$egui_versions" != "v0.36.2" ]]; then
  printf 'Expected only egui v0.36.2, found: %s\n' "$egui_versions" >&2
  exit 1
fi
