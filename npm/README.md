# retrochat

LLM Agent Chat History Retrospect Application

A powerful TUI (Terminal User Interface) application for analyzing and retrospecting on your LLM chat history from various providers like Claude, Gemini, and more.

## Installation

```bash
# Install globally via npm
npm install -g @sanggggg/retrochat

# Or use with npx
npx @sanggggg/retrochat
```

## Quick Start

```bash
# Launch the TUI interface (default mode)
retrochat

# Import chat history from providers
retrochat sync claude gemini

# Watch for new chats
retrochat sync all -w --verbose

# Run AI analysis (requires GOOGLE_AI_API_KEY)
retrochat analysis run --all
```

## Features

- 📊 **TUI Interface**: Beautiful terminal interface for browsing chat history
- 🔍 **Multi-Provider Support**: Import from Claude Code, Gemini, Codex, and more
- 📈 **Query & Search**: Browse sessions, search messages, and filter by provider
- 🔄 **Auto-Watch**: Automatically sync new chats in real-time
- 🤖 **AI-Powered Analysis**: Analyze sessions with Google AI assistance

## Requirements

- Node.js 16 or higher

## Supported Platforms

- macOS (Intel & Apple Silicon)
- Linux (x64 & ARM64)
- Windows (x64)

## Documentation

For more information, visit the [GitHub repository](https://github.com/wafflestudio/retrochat).

## License

MIT
