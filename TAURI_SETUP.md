# Tauri Desktop Application Setup

This document describes how to build and run the Retrochat desktop application using Tauri.

## Project Structure

```
retrochat/
├── src/                    # Original CLI/TUI Rust code
├── src-tauri/             # Tauri backend
│   ├── src/
│   │   ├── main.rs        # Tauri application entry point
│   │   └── lib.rs         # Library exports
│   ├── Cargo.toml         # Tauri dependencies
│   ├── tauri.conf.json    # Tauri configuration
│   └── build.rs           # Build script
├── ui/                    # Frontend (HTML/CSS/JS)
│   ├── index.html
│   ├── styles.css
│   └── app.js
└── Cargo.toml             # Main project dependencies
```

## Prerequisites

1. **Rust** - Already installed (1.75+)
2. **Node.js** - Not required for this setup (we're using vanilla HTML/CSS/JS)
3. **System dependencies** - Required for Tauri

### macOS
```bash
xcode-select --install
```

### Linux (Debian/Ubuntu)
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

### Windows
No additional dependencies required beyond Rust and Visual Studio Build Tools.

## Installation

### Install Tauri CLI

```bash
cargo install tauri-cli --version 2.1
```

Or use the project-specific installation:

```bash
cd src-tauri
cargo install tauri-cli
```

## Development

### Running in Development Mode

From the project root:

```bash
cd src-tauri
cargo tauri dev
```

This will:
1. Build the Rust backend
2. Serve the frontend from the `ui/` directory
3. Open the application window

### Building for Production

```bash
cd src-tauri
cargo tauri build
```

The built application will be in `src-tauri/target/release/bundle/`.

## Available Tauri Commands

The Tauri backend exposes the following commands that can be called from the frontend:

### `get_sessions`
Retrieves a paginated list of chat sessions.

**Parameters:**
- `page: Option<i32>` - Page number (default: 1)
- `page_size: Option<i32>` - Items per page (default: 20)
- `provider: Option<String>` - Filter by provider (Claude, Gemini, Codex)

**Returns:** `Vec<SessionListItem>`

### `get_session_detail`
Retrieves detailed information about a specific session including all messages.

**Parameters:**
- `session_id: String` - UUID of the session

**Returns:** `SessionDetail`

### `search_messages`
Searches through all messages by content.

**Parameters:**
- `query: String` - Search query
- `limit: Option<i32>` - Maximum results (default: 20)

**Returns:** `Vec<SearchResult>`

### `get_providers`
Returns the list of available providers.

**Returns:** `Vec<String>`

## Frontend Development

The frontend is built with vanilla HTML/CSS/JavaScript and uses the Tauri API to communicate with the backend.

### Key Files

- **`ui/index.html`** - Main HTML structure
- **`ui/styles.css`** - Styling and layout
- **`ui/app.js`** - Application logic and Tauri command invocations

### Using Tauri Commands in JavaScript

```javascript
// Import Tauri API
const { invoke } = window.__TAURI__.core;

// Call a Tauri command
const sessions = await invoke('get_sessions', {
    page: 1,
    pageSize: 20,
    provider: 'Claude',
});
```

## Architecture

The Tauri application maintains the existing architecture:

```
UI (HTML/CSS/JS)
    ↓ (Tauri Commands)
Tauri Backend (main.rs)
    ↓
Retrochat Services
    ↓
Database (SQLite)
```

The Tauri backend is a thin wrapper around the existing `retrochat` library, exposing the query and analysis services through Tauri commands.

## Dual Interface Support

Retrochat now supports **both** interfaces:

1. **TUI (Terminal)** - Original terminal interface using Ratatui
   ```bash
   cargo run
   ```

2. **GUI (Desktop)** - New Tauri desktop application
   ```bash
   cd src-tauri && cargo tauri dev
   ```

Both interfaces use the same underlying services and database, so data is shared between them.

## Configuration

### Tauri Configuration (`src-tauri/tauri.conf.json`)

Key settings:
- `productName`: "Retrochat"
- `identifier`: "com.retrochat.app"
- `frontendDist`: "../ui" (points to the UI directory)
- Window settings (size, title, etc.)

### Application Icons

To add custom icons:

1. Create a source icon (1024x1024 PNG recommended)
2. Generate icon set:
   ```bash
   cd src-tauri
   cargo tauri icon /path/to/your/icon.png
   ```
3. Update `tauri.conf.json` to enable bundling:
   ```json
   "bundle": {
     "active": true,
     "targets": "all",
     "icon": [
       "icons/32x32.png",
       "icons/128x128.png",
       "icons/128x128@2x.png",
       "icons/icon.icns",
       "icons/icon.ico"
     ]
   }
   ```

## Troubleshooting

### Build Errors

If you encounter build errors:

1. Ensure all system dependencies are installed
2. Update Rust: `rustup update`
3. Clean build artifacts: `cargo clean`
4. Try building again

### Database Connection Issues

The Tauri app uses the same database path as the CLI/TUI:
- Default: `~/.retrochat/retrochat.db`

Ensure the database exists by running the TUI at least once:
```bash
cargo run
```

### Frontend Not Loading

If the frontend doesn't load:

1. Check that the `ui/` directory exists with all files
2. Verify `frontendDist` in `tauri.conf.json` points to "../ui"
3. Check browser console for errors (DevTools: Cmd+Option+I on macOS)

## Features

The desktop application provides:

- Session browsing with pagination
- Session detail view with all messages
- Message search functionality
- Provider filtering
- Clean, modern UI
- Responsive layout

## Future Enhancements

Potential improvements:

1. Add analytics/AI analysis features to GUI
2. Implement real-time sync updates
3. Add export functionality
4. Theme customization (dark mode)
5. Custom icon design
6. Multi-window support
7. Keyboard shortcuts

## Testing

To test the application:

1. Ensure you have chat data imported:
   ```bash
   cargo run -- sync all
   ```

2. Run the Tauri dev server:
   ```bash
   cd src-tauri
   cargo tauri dev
   ```

3. Test all features:
   - Browse sessions
   - View session details
   - Search messages
   - Filter by provider
   - Navigate with pagination

## Resources

- [Tauri Documentation](https://tauri.app/)
- [Tauri API Reference](https://tauri.app/v2/reference/)
- [Retrochat Main README](./README.md)
