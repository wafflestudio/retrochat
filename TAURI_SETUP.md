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
├── ui-react/              # Frontend (React + Vite + shadcn/ui)
│   ├── src/
│   │   ├── components/    # React components
│   │   ├── hooks/         # Custom React hooks
│   │   ├── utils/         # Utility functions
│   │   ├── App.jsx        # Main application component
│   │   └── main.jsx       # Application entry point
│   ├── package.json       # Node dependencies
│   └── vite.config.js     # Vite configuration
├── ui/                    # Legacy Frontend (HTML/CSS/JS) - kept for reference
└── Cargo.toml             # Main project dependencies
```

## Prerequisites

1. **Rust** - Already installed (1.75+)
2. **Node.js** - Required (v18+ recommended) for the React frontend
3. **npm** - Comes with Node.js
4. **System dependencies** - Required for Tauri

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

### First-time Setup

Install the React frontend dependencies:

```bash
cd ui-react
npm install
```

### Running in Development Mode

From the project root:

```bash
cd src-tauri
cargo tauri dev
```

This will:
1. Start the Vite development server (React frontend with HMR)
2. Build the Rust backend
3. Open the application window with hot-reload enabled

**Note**: The Tauri config is set up to automatically start the Vite dev server, so you don't need to run it separately.

### Building for Production

```bash
cd src-tauri
cargo tauri build
```

This will:
1. Build the React frontend for production (`ui-react/dist`)
2. Build the Tauri application
3. Create platform-specific bundles

The built application will be in `src-tauri/target/release/bundle/`.

### Manual Frontend Development

If you want to develop the frontend separately:

```bash
cd ui-react
npm run dev
```

This starts the Vite dev server on `http://localhost:5173`.

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

The frontend is built with **React**, **Vite**, **Tailwind CSS**, and **shadcn/ui** components. It uses the Tauri API to communicate with the Rust backend.

### Technology Stack

- **React 18** - UI framework
- **Vite** - Build tool and dev server (fast HMR)
- **Tailwind CSS v4** - Utility-first CSS framework
- **shadcn/ui** - High-quality, accessible component library
- **Lucide React** - Icon library
- **@tauri-apps/api** - Tauri JavaScript bindings

### Key Directories

- **`ui-react/src/components/`** - React components (SessionList, SessionDetail, SearchModal, UI components)
- **`ui-react/src/hooks/`** - Custom React hooks (useSessions, useSessionDetail, useSearch)
- **`ui-react/src/utils/`** - Utility functions (Tauri API wrappers, formatters)
- **`ui-react/src/App.jsx`** - Main application component

### Using Tauri Commands in React

```javascript
// Import from utils/tauri.js wrapper
import { getSessions, getSessionDetail, searchMessages } from './utils/tauri';

// In a React component or hook
const sessions = await getSessions(page, pageSize, provider);
const detail = await getSessionDetail(sessionId);
const results = await searchMessages(query, limit);
```

### Component Architecture

The application uses a clean component hierarchy:

```
App.jsx (main container)
├── SessionList (sidebar with pagination)
│   └── SessionItem (individual session cards)
├── SessionDetail (main content area)
│   └── Message (individual message display)
└── SearchModal (search overlay)
    └── SearchResultItem (search result cards)
```

### Styling with shadcn/ui

Components use shadcn/ui primitives for consistent design:
- `Button` - Interactive buttons with variants
- `Card` - Content containers
- `Input` - Form inputs
- `Select` - Dropdown selects
- `Badge` - Status indicators
- `Dialog` - Modal overlays

All components are styled with Tailwind CSS utilities and support theming through CSS variables.

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
