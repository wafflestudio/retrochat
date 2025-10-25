# retrochat

LLM Agent Chat History Retrospect Application

A powerful TUI (Terminal User Interface) application for analyzing and retrospecting on your LLM chat history from various providers like Claude, Cursor, Gemini, and more.

## Installation

```bash
# Install globally via npm
npm install -g @sanggggg/retrochat

# Or use with npx
npx @sanggggg/retrochat tui
```

## Quick Start

```bash
# Launch the TUI interface
retrochat tui

# Import chat history from providers
retrochat import claude cursor

# Watch for new chats
retrochat watch all --verbose

# Analyze your chat sessions
retrochat analyze insights
```

## Features

- 📊 **TUI Interface**: Beautiful terminal interface for browsing chat history
- 🔍 **Multi-Provider Support**: Import from Claude, Cursor, Gemini, and more
- 📈 **Analytics**: Generate insights and export data
- 🔄 **Auto-Watch**: Automatically import new chats
- 🤖 **AI-Powered Retrospection**: Analyze sessions with AI assistance

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
