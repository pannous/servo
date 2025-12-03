#!/bin/bash
# Servo Build Script with TypeScript & WebAssembly Support

set -e  # Exit on error

echo "🚀 Building Servo with TypeScript and WASM support..."
echo "⚙️  SERVO_ENABLE_MEDIA=0 (to avoid slow GStreamer)"

export SERVO_ENABLE_MEDIA=0

# Unmount any existing Servo volumes
diskutil unmountDisk force /Volumes/Servo/ 2>/dev/null || true
diskutil unmountDisk force "/Volumes/Servo 1/" 2>/dev/null || true

# Clean up old DMG if using debug build
if [ "$1" == "--release" ]; then
    echo "📦 Building RELEASE version..."
    rm -f /opt/cargo/release/servo-tech-demo.dmg
    ./mach build --release
    ./mach package --release
    DMG_PATH="/opt/cargo/release/servo-tech-demo.dmg"
else
    echo "📦 Building DEBUG version..."
    rm -f /opt/cargo/debug/servo-tech-demo.dmg
    ./mach build
    ./mach package
    DMG_PATH="/opt/cargo/debug/servo-tech-demo.dmg"
fi

# Kill any running Servo instances
killall -SIGKILL servo 2>/dev/null || true

# Mount DMG
echo "💿 Mounting DMG..."
MOUNT_POINT=$(hdiutil attach "$DMG_PATH" | grep Volumes | awk '{print $NF}')

# Install to Applications
echo "📲 Installing to /Applications..."
cp -R "$MOUNT_POINT/Servo.app" /Applications/

# Unmount DMG
hdiutil detach "$MOUNT_POINT"

echo "✅ Build complete! Servo installed to /Applications/Servo.app"
echo ""
echo "To test TypeScript: open -a /Applications/Servo.app test-typescript.html"
echo "To test WebAssembly: open -a /Applications/Servo.app test-wasm.html"
