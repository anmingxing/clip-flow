#!/usr/bin/env bash
set -e

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_DIR"

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
fi

echo "==> Installing frontend dependencies if needed..."
if [ ! -d "node_modules" ]; then
    npm install
fi

echo "==> Building ClipFlow (Tauri v2 Release mode)..."
npm run tauri -- build

BUNDLE_DIR="$PROJECT_DIR/src-tauri/target/release/bundle"
OUT_DIR="$PROJECT_DIR/build"
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

if [ -d "$BUNDLE_DIR/macos/ClipFlow.app" ]; then
    cp -R "$BUNDLE_DIR/macos/ClipFlow.app" "$OUT_DIR/ClipFlow.app"
    echo "==> Copied ClipFlow.app to $OUT_DIR/ClipFlow.app"
fi

if [ -d "$BUNDLE_DIR/dmg" ]; then
    cp "$BUNDLE_DIR/dmg/"*.dmg "$OUT_DIR/" 2>/dev/null || true
fi

if [ -d "$BUNDLE_DIR/appimage" ]; then
    cp "$BUNDLE_DIR/appimage/"*.AppImage "$OUT_DIR/" 2>/dev/null || true
fi

if [ -d "$BUNDLE_DIR/deb" ]; then
    cp "$BUNDLE_DIR/deb/"*.deb "$OUT_DIR/" 2>/dev/null || true
fi

echo "==> Build artifacts available in $OUT_DIR:"
ls -lh "$OUT_DIR"
