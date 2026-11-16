#!/usr/bin/env bash
set -e

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_DIR"

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
fi

VERSION="${1:-v1.0.0}"
echo "==> Building ClipFlow Release (Version: $VERSION)..."

if [ ! -d "node_modules" ]; then
    npm install
fi

npm run tauri -- build

BUNDLE_DIR="$PROJECT_DIR/src-tauri/target/release/bundle"
OUT_DIR="$PROJECT_DIR/build"
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

OS_NAME="$(uname -s)"
if [ "$OS_NAME" = "Darwin" ]; then
    if [ -d "$BUNDLE_DIR/macos/ClipFlow.app" ]; then
        cp -R "$BUNDLE_DIR/macos/ClipFlow.app" "$OUT_DIR/ClipFlow.app"
        (cd "$OUT_DIR" && zip -r -y "ClipFlow-${VERSION}-macos.zip" ClipFlow.app)
    fi

    DMG_FILE=$(find "$BUNDLE_DIR/dmg" -maxdepth 1 -name "*.dmg" 2>/dev/null | head -n 1 || true)
    if [ -n "$DMG_FILE" ]; then
        cp "$DMG_FILE" "$OUT_DIR/ClipFlow-${VERSION}-macos.dmg"
    elif [ -d "$OUT_DIR/ClipFlow.app" ]; then
        mkdir -p "$OUT_DIR/dmg_staging"
        cp -R "$OUT_DIR/ClipFlow.app" "$OUT_DIR/dmg_staging/"
        ln -s /Applications "$OUT_DIR/dmg_staging/Applications"
        hdiutil create -volname "ClipFlow" -srcfolder "$OUT_DIR/dmg_staging" -ov -format UDZO "$OUT_DIR/ClipFlow-${VERSION}-macos.dmg"
        rm -rf "$OUT_DIR/dmg_staging"
    fi

    (cd "$OUT_DIR" && shasum -a 256 ClipFlow-* > "ClipFlow-${VERSION}-sha256.txt")
else
    find "$BUNDLE_DIR" -maxdepth 2 -type f \( -name "*.AppImage" -o -name "*.deb" -o -name "*.rpm" -o -name "*.msi" -o -name "*.exe" \) -exec cp {} "$OUT_DIR/" \;
    (cd "$OUT_DIR" && sha256sum * > "ClipFlow-${VERSION}-sha256.txt" 2>/dev/null || shasum -a 256 * > "ClipFlow-${VERSION}-sha256.txt")
fi

echo "==> Done! Release artifacts created in $OUT_DIR:"
ls -lh "$OUT_DIR"
