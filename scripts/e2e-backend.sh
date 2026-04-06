#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$ROOT_DIR/.tmp/playwright"
DB_PATH="$TMP_DIR/substore.db"

mkdir -p "$TMP_DIR"
rm -f "$DB_PATH" "$DB_PATH-shm" "$DB_PATH-wal"

export HOST=127.0.0.1
export PORT=18080
export WEB_DIST_DIR="$ROOT_DIR/web/dist"
export DATABASE_URL="sqlite://$DB_PATH?mode=rwc"
export COOKIE_SECURE=false
export ADMIN_USER=admin
export ADMIN_PASSWORD=admin
export FETCH_HOST_OVERRIDES="example.test:19081=127.0.0.1:19081"
export RUST_LOG=warn

cd "$ROOT_DIR"
exec cargo run -p submora
