#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
if ! command -v cargo >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo 'Falta Rust. Instala rustup desde https://rustup.rs antes de continuar.' >&2
  exit 1
fi
exec ./node_modules/.bin/tauri "$@"
