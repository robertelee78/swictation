# Tauri macOS Icon Generation

## Current Status

The `tauri-ui/src-tauri/icons/icon.icns` file currently exists but is empty (0 bytes).

## Generating icon.icns from PNG

### Option 1: Using iconutil (macOS only)

```bash
# Create iconset directory
mkdir icon.iconset

# Copy and resize PNGs (requires imagemagick)
convert tauri-ui/src-tauri/icons/icon.png -resize 16x16 icon.iconset/icon_16x16.png
convert tauri-ui/src-tauri/icons/icon.png -resize 32x32 icon.iconset/icon_16x16@2x.png
convert tauri-ui/src-tauri/icons/icon.png -resize 32x32 icon.iconset/icon_32x32.png
convert tauri-ui/src-tauri/icons/icon.png -resize 64x64 icon.iconset/icon_32x32@2x.png
convert tauri-ui/src-tauri/icons/icon.png -resize 128x128 icon.iconset/icon_128x128.png
convert tauri-ui/src-tauri/icons/icon.png -resize 256x256 icon.iconset/icon_128x128@2x.png
convert tauri-ui/src-tauri/icons/icon.png -resize 256x256 icon.iconset/icon_256x256.png
convert tauri-ui/src-tauri/icons/icon.png -resize 512x512 icon.iconset/icon_256x256@2x.png
convert tauri-ui/src-tauri/icons/icon.png -resize 512x512 icon.iconset/icon_512x512.png
convert tauri-ui/src-tauri/icons/icon.png -resize 1024x1024 icon.iconset/icon_512x512@2x.png

# Generate .icns file
iconutil -c icns icon.iconset -o tauri-ui/src-tauri/icons/icon.icns

# Cleanup
rm -rf icon.iconset
```

### Option 2: Using png2icns (Linux/macOS)

```bash
# Install png2icns
brew install libicns  # macOS
sudo apt install libicns-utils  # Ubuntu

# Generate from 1024x1024 PNG
png2icns tauri-ui/src-tauri/icons/icon.icns tauri-ui/src-tauri/icons/icon.png
```

### Option 3: Tauri CLI (Automatic)

Tauri CLI can automatically generate icons during build:

```bash
# Install Tauri CLI
npm install -g @tauri-apps/cli

# Generate all icons from a single PNG (1024x1024 recommended)
tauri icon tauri-ui/src-tauri/icons/icon.png
```

This will generate all required formats including .icns for macOS.

## Required Icon Sizes

For best results, start with a 1024x1024px PNG and generate:
- 16x16, 32x32, 64x64, 128x128, 256x256, 512x512, 1024x1024
- Include @2x retina variants (32x32@2x, 64x64@2x, etc.)

## Current Configuration

The `tauri.conf.json` is already configured to use icon.icns:

```json
{
  "bundle": {
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "macOS": {
      "minimumSystemVersion": "14.0",
      "frameworks": [],
      "exceptionDomain": ""
    }
  }
}
```

## Building for macOS

Once icon.icns is generated:

```bash
cd tauri-ui
npm run tauri build
```

This will create:
- `src-tauri/target/release/bundle/macos/Swictation.app` - macOS app bundle
- `src-tauri/target/release/bundle/dmg/Swictation_0.1.0_aarch64.dmg` - DMG installer

## Notes

- The current icon.png (128x128) should be replaced with a 1024x1024 version for best quality
- macOS requires .icns format for proper app bundle icon display
- Tauri will fall back to PNG if .icns is missing, but with lower quality
- DMG background and appearance can be customized via tauri.conf.json
