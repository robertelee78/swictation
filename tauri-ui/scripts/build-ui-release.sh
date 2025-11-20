#!/bin/bash
set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Required event names that MUST be in the bundle
REQUIRED_EVENTS=(
    "metrics-connected"
    "metrics-update"
    "session-start"
    "session-end"
    "state-change"
    "transcription"
)

echo -e "${BLUE}🔨 Building Swictation Tauri UI for release...${NC}"
echo ""

# Get to the tauri-ui directory
cd "$(dirname "$0")/.."
TAURI_UI_DIR=$(pwd)
NPM_PACKAGE_DIR="${TAURI_UI_DIR}/../npm-package"

# Step 1: Clean build directories
echo -e "${YELLOW}📦 Step 1: Cleaning build directories...${NC}"
rm -rf dist
rm -rf src-tauri/target/release/bundle
echo -e "${GREEN}✓ Clean complete${NC}"
echo ""

# Step 2: TypeScript compilation check
echo -e "${YELLOW}🔍 Step 2: TypeScript compilation check...${NC}"
npx tsc --noEmit
if [ $? -ne 0 ]; then
    echo -e "${RED}❌ TypeScript compilation failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ TypeScript check passed${NC}"
echo ""

# Step 3: Build frontend with Vite
echo -e "${YELLOW}📦 Step 3: Building frontend (Vite)...${NC}"
npm run build
if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Frontend build failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Frontend build complete${NC}"
echo ""

# Step 4: Verify bundled JavaScript contains all event names
echo -e "${YELLOW}🔍 Step 4: Verifying bundled JavaScript...${NC}"

# Find the main JS bundle
BUNDLE_JS=$(find dist/assets -name "index-*.js" | head -1)
if [ -z "$BUNDLE_JS" ]; then
    echo -e "${RED}❌ No bundled JavaScript found in dist/assets/${NC}"
    exit 1
fi

echo "Checking bundle: $BUNDLE_JS"

MISSING_EVENTS=()
for event in "${REQUIRED_EVENTS[@]}"; do
    if ! grep -q "\"$event\"" "$BUNDLE_JS"; then
        MISSING_EVENTS+=("$event")
    fi
done

if [ ${#MISSING_EVENTS[@]} -gt 0 ]; then
    echo -e "${RED}❌ Missing event names in bundle:${NC}"
    for event in "${MISSING_EVENTS[@]}"; do
        echo -e "${RED}  - $event${NC}"
    done
    echo ""
    echo -e "${RED}This indicates a Vite caching issue. Bundle does not match source code.${NC}"
    exit 1
fi

echo -e "${GREEN}✓ All ${#REQUIRED_EVENTS[@]} event names found in bundle${NC}"
for event in "${REQUIRED_EVENTS[@]}"; do
    echo -e "${GREEN}  ✓ $event${NC}"
done
echo ""

# Step 5: Frontend bundle size validation
echo -e "${YELLOW}📊 Step 5: Validating bundle sizes...${NC}"
BUNDLE_SIZE_MB=$(du -sm dist | cut -f1)
if [ "$BUNDLE_SIZE_MB" -gt 10 ]; then
    echo -e "${YELLOW}⚠️  Warning: Bundle size is ${BUNDLE_SIZE_MB}MB (>10MB threshold)${NC}"
else
    echo -e "${GREEN}✓ Bundle size: ${BUNDLE_SIZE_MB}MB${NC}"
fi
echo ""

# Step 6: Build Tauri binary
echo -e "${YELLOW}📦 Step 6: Building Tauri binary (Rust + packaging)...${NC}"
npm run tauri build
if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Tauri build failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Tauri build complete${NC}"
echo ""

# Step 7: Binary size check
echo -e "${YELLOW}📊 Step 7: Validating binary size...${NC}"
BINARY_SIZE_MB=$(du -sm "$BINARY_PATH" | cut -f1)
if [ "$BINARY_SIZE_MB" -gt 20 ]; then
    echo -e "${YELLOW}⚠️  Warning: Binary size is ${BINARY_SIZE_MB}MB (>20MB threshold)${NC}"
else
    echo -e "${GREEN}✓ Binary size: ${BINARY_SIZE_MB}MB${NC}"
fi
echo ""

# Step 8: Verify binary contains all event names
echo -e "${YELLOW}🔍 Step 8: Verifying binary contains event names...${NC}"

BINARY_PATH="src-tauri/target/release/swictation-ui"
if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}❌ Binary not found at $BINARY_PATH${NC}"
    exit 1
fi

BINARY_MISSING=()
for event in "${REQUIRED_EVENTS[@]}"; do
    if ! strings "$BINARY_PATH" | grep -q "^$event$"; then
        BINARY_MISSING+=("$event")
    fi
done

if [ ${#BINARY_MISSING[@]} -gt 0 ]; then
    echo -e "${RED}❌ Missing event names in binary:${NC}"
    for event in "${BINARY_MISSING[@]}"; do
        echo -e "${RED}  - $event${NC}"
    done
    exit 1
fi

echo -e "${GREEN}✓ All ${#REQUIRED_EVENTS[@]} event names found in binary${NC}"
echo ""

# Step 9: Copy verified binary to npm package
echo -e "${YELLOW}📋 Step 9: Copying binary to npm package...${NC}"

if [ ! -d "$NPM_PACKAGE_DIR/bin" ]; then
    echo -e "${RED}❌ npm-package/bin directory not found${NC}"
    exit 1
fi

cp "$BINARY_PATH" "$NPM_PACKAGE_DIR/bin/swictation-ui"
chmod +x "$NPM_PACKAGE_DIR/bin/swictation-ui"
echo -e "${GREEN}✓ Binary copied to $NPM_PACKAGE_DIR/bin/swictation-ui${NC}"
echo ""

# Step 10: Create checksums
echo -e "${YELLOW}🔐 Step 10: Creating SHA256 checksums...${NC}"

CHECKSUM_FILE="${TAURI_UI_DIR}/build-checksums.txt"
rm -f "$CHECKSUM_FILE"

echo "# Swictation Tauri UI Build Checksums" > "$CHECKSUM_FILE"
echo "# Generated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")" >> "$CHECKSUM_FILE"
echo "" >> "$CHECKSUM_FILE"

# Checksum the source binary
BINARY_CHECKSUM=$(sha256sum "$BINARY_PATH")
echo "$BINARY_CHECKSUM" >> "$CHECKSUM_FILE"

# Checksum the npm package binary
NPM_BINARY_CHECKSUM=$(sha256sum "$NPM_PACKAGE_DIR/bin/swictation-ui")
echo "$NPM_BINARY_CHECKSUM" >> "$CHECKSUM_FILE"

# Checksum the bundled JS
BUNDLE_CHECKSUM=$(sha256sum "$BUNDLE_JS")
echo "$BUNDLE_CHECKSUM" >> "$CHECKSUM_FILE"

# Checksum the bundles
sha256sum src-tauri/target/release/bundle/deb/*.deb >> "$CHECKSUM_FILE" 2>/dev/null || true
sha256sum src-tauri/target/release/bundle/rpm/*.rpm >> "$CHECKSUM_FILE" 2>/dev/null || true
sha256sum src-tauri/target/release/bundle/appimage/*.AppImage >> "$CHECKSUM_FILE" 2>/dev/null || true

echo -e "${GREEN}✓ Checksums saved to $CHECKSUM_FILE${NC}"
echo ""

# Step 11: Generate build manifest
echo -e "${YELLOW}📝 Step 11: Generating build manifest...${NC}"

MANIFEST_FILE="/tmp/tauri-ui-build-manifest.json"
GIT_COMMIT=$(git -C "$TAURI_UI_DIR" rev-parse HEAD 2>/dev/null || echo "unknown")
GIT_BRANCH=$(git -C "$TAURI_UI_DIR" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
BUILD_TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

cat > "$MANIFEST_FILE" << EOF
{
  "build": {
    "timestamp": "$BUILD_TIMESTAMP",
    "git_commit": "$GIT_COMMIT",
    "git_branch": "$GIT_BRANCH"
  },
  "verification": {
    "typescript_check": "passed",
    "event_names_verified": true,
    "required_events": [
$(printf '      "%s"' "${REQUIRED_EVENTS[0]}"
for event in "${REQUIRED_EVENTS[@]:1}"; do
    printf ',\n      "%s"' "$event"
done
printf '\n')
    ]
  },
  "artifacts": {
    "bundle_js": {
      "path": "$BUNDLE_JS",
      "size_mb": $BUNDLE_SIZE_MB,
      "checksum": "$(echo $BUNDLE_CHECKSUM | cut -d' ' -f1)"
    },
    "binary": {
      "path": "$BINARY_PATH",
      "size_mb": $BINARY_SIZE_MB,
      "checksum": "$(echo $BINARY_CHECKSUM | cut -d' ' -f1)"
    },
    "npm_binary": {
      "path": "$NPM_PACKAGE_DIR/bin/swictation-ui",
      "checksum": "$(echo $NPM_BINARY_CHECKSUM | cut -d' ' -f1)"
    }
  }
}
EOF

echo -e "${GREEN}✓ Build manifest saved to $MANIFEST_FILE${NC}"
echo ""

# Summary
echo -e "${GREEN}✅ Tauri UI build complete!${NC}"
echo ""
echo -e "${BLUE}📊 Build Summary:${NC}"
echo -e "  Binary: $(ls -lh $BINARY_PATH | awk '{print $5}')"
echo -e "  Bundle JS: $(ls -lh $BUNDLE_JS | awk '{print $5}')"
echo -e "  npm package binary: $(ls -lh $NPM_PACKAGE_DIR/bin/swictation-ui | awk '{print $5}')"
echo ""
echo -e "${BLUE}📦 Bundles created:${NC}"
ls -1 src-tauri/target/release/bundle/deb/*.deb 2>/dev/null || echo "  (no .deb)"
ls -1 src-tauri/target/release/bundle/rpm/*.rpm 2>/dev/null || echo "  (no .rpm)"
ls -1 src-tauri/target/release/bundle/appimage/*.AppImage 2>/dev/null || echo "  (no .AppImage)"
echo ""
echo -e "${GREEN}✓ Ready for npm publish!${NC}"
