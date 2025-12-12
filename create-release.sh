#!/bin/bash
set -e

VERSION="${1:-$(date +%Y.%m.%d)}"
ARCH=$(uname -m)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
RELEASE_NAME="servo-${VERSION}-${OS}-${ARCH}"
TARBALL="${RELEASE_NAME}.tar.gz"

echo "Creating release: ${RELEASE_NAME}"

# Ensure release binary exists
if [ ! -f "target/release/servo" ]; then
    echo "Error: target/release/servo not found"
    echo "Run: ./mach build --release"
    exit 1
fi

# Create release directory
rm -rf "/tmp/${RELEASE_NAME}"
mkdir -p "/tmp/${RELEASE_NAME}"

# Copy binary and essential files
cp target/release/servo "/tmp/${RELEASE_NAME}/"
cp README.md "/tmp/${RELEASE_NAME}/"
cp -r resources "/tmp/${RELEASE_NAME}/" 2>/dev/null || true

# Create tarball
cd /tmp
tar -czf "${TARBALL}" "${RELEASE_NAME}"
TARBALL_PATH="/tmp/${TARBALL}"

# Calculate SHA256
SHA256=$(shasum -a 256 "${TARBALL}" | cut -d' ' -f1)

echo ""
echo "✅ Release created: ${TARBALL_PATH}"
echo "   SHA256: ${SHA256}"
echo ""
echo "To create GitHub release:"
echo "  gh release create v${VERSION} ${TARBALL_PATH} --title \"Servo ${VERSION}\" --notes \"Binary release for ${OS}/${ARCH}\""
echo ""
echo "Formula snippet:"
echo "  url \"https://github.com/pannous/servo/releases/download/v${VERSION}/${TARBALL}\""
echo "  sha256 \"${SHA256}\""
