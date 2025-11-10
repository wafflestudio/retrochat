# Icons for Tauri Application

## Generate Icons

You need to generate application icons for Tauri. You can use the Tauri icon generator:

```bash
# Install cargo-tauri if not already installed
cargo install tauri-cli

# Generate icons from a source PNG (1024x1024 recommended)
cargo tauri icon /path/to/your/icon.png
```

Alternatively, you can use an online tool or create icons manually:

Required icon files:
- `32x32.png` - Small icon (32x32 pixels)
- `128x128.png` - Medium icon (128x128 pixels)
- `128x128@2x.png` - Retina medium icon (256x256 pixels)
- `icon.icns` - macOS icon
- `icon.ico` - Windows icon

## Temporary Solution

For development, you can temporarily comment out the `icon` field in `tauri.conf.json` or use default icons.
