#!/bin/bash
set -e

APP_NAME="Language Handler"
BUNDLE_DIR="target/release/bundle/osx/$APP_NAME.app"
CONTENTS_DIR="$BUNDLE_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

echo "🚀 Building Release Binaries (ARM64)..."
cargo build --release --target aarch64-apple-darwin -p lh-gui
cargo build --release --target aarch64-apple-darwin -p lh-daemon

echo "📂 Creating App Bundle Structure..."
rm -rf "$BUNDLE_DIR"
mkdir -p "$MACOS_DIR"
mkdir -p "$RESOURCES_DIR"

echo "📦 Copying Binaries..."
cp target/aarch64-apple-darwin/release/lh-gui "$MACOS_DIR/$APP_NAME"
cp target/aarch64-apple-darwin/release/lh-daemon "$MACOS_DIR/lh-daemon"

echo "🎨 Copying Assets..."
if [ -f "gui/assets/icon.icns" ]; then
    cp "gui/assets/icon.icns" "$RESOURCES_DIR/AppIcon.icns"
else
    echo "⚠️ Warning: Icon not found at gui/assets/icon.icns"
fi

echo "📝 Creating Info.plist..."
cat > "$CONTENTS_DIR/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>$APP_NAME</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIdentifier</key>
    <string>com.languagehandler.app</string>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

echo "✍️  Signing Bundle..."
codesign --force --deep --sign - "$BUNDLE_DIR"

echo "✅ Done! App is at: $BUNDLE_DIR"
open "target/release/bundle/osx"
