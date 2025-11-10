# Tauri React Migration Plan

## Overview

This document outlines the migration plan from the current vanilla HTML/CSS/JavaScript Tauri frontend to a modern React-based frontend using Vite as the build tool.

## Current State

The current UI (`ui/` directory) consists of:
- `index.html` - Main HTML structure
- `styles.css` - Styling and layout
- `app.js` - Application logic with vanilla JavaScript

## Goals

1. Migrate to React for better component organization and maintainability
2. Use Vite for fast development and optimized builds
3. Maintain all existing functionality (session browsing, search, detail view)
4. Improve code structure with proper component separation
5. Keep the same Tauri backend interface (no backend changes needed)

## Migration Steps

### 1. Project Setup

Create a new React + Vite project structure:
- Initialize Vite with React template in `ui-react/` directory
- Install necessary dependencies:
  - `react` & `react-dom`
  - `@tauri-apps/api` for Tauri integration
  - Development tools (Vite, ESLint, etc.)

### 2. Component Architecture

Migrate functionality into React components:

```
ui-react/
├── src/
│   ├── App.jsx                 # Main application component
│   ├── components/
│   │   ├── SessionList.jsx     # Session list with pagination
│   │   ├── SessionItem.jsx     # Individual session item
│   │   ├── SessionDetail.jsx   # Session detail view
│   │   ├── Message.jsx         # Individual message component
│   │   ├── SearchBar.jsx       # Search input and button
│   │   └── SearchModal.jsx     # Search results modal
│   ├── hooks/
│   │   ├── useSessions.js      # Hook for session data fetching
│   │   ├── useSearch.js        # Hook for search functionality
│   │   └── usePagination.js    # Hook for pagination logic
│   ├── utils/
│   │   ├── tauri.js            # Tauri API wrapper functions
│   │   └── formatters.js       # Date/time formatting utilities
│   ├── styles/
│   │   └── App.css             # Global and component styles
│   └── main.jsx                # Application entry point
├── index.html                   # HTML template
├── package.json
└── vite.config.js              # Vite configuration
```

### 3. Feature Implementation

Migrate features from vanilla JS to React:

#### Session List
- Paginated session browsing
- Provider filtering
- Click to view details
- Active session highlighting

#### Session Detail
- Display session metadata
- Show all messages with proper formatting
- Role-based message styling

#### Search
- Modal-based search interface
- Real-time search results
- Click result to navigate to session

### 4. Styling

- Port existing CSS to React-friendly approach
- Consider CSS modules or styled-components for component-scoped styles
- Maintain current design and UX

### 5. Tauri Integration

Update Tauri configuration:
- Point `frontendDist` to React build output (`ui-react/dist`)
- Update development server configuration
- Ensure hot reload works in development mode

### 6. Testing

- Test all existing functionality
- Verify Tauri commands work correctly
- Ensure pagination, filtering, and search work as expected
- Test in development and production builds

### 7. Documentation

Update documentation:
- Modify `TAURI_SETUP.md` with React setup instructions
- Add development workflow for React
- Document component architecture

## Benefits

1. **Better Code Organization**: Components are modular and reusable
2. **Improved Maintainability**: React's component model makes code easier to understand
3. **Modern Developer Experience**: Vite provides fast HMR and optimized builds
4. **State Management**: React hooks provide clean state management
5. **Type Safety (Future)**: Easy path to TypeScript migration if needed

## Backward Compatibility

- Keep the old `ui/` directory as `ui-legacy/` for reference
- Backend Tauri commands remain unchanged
- Same database and data layer

## Timeline

Estimated completion: 1-2 hours for full migration and testing

## Rollback Plan

If issues arise:
1. Revert Tauri config changes
2. Point back to `ui-legacy/` directory
3. Original functionality is preserved

## Success Criteria

- [ ] All existing features work in React version
- [ ] Development mode works with hot reload
- [ ] Production build completes successfully
- [ ] UI/UX matches or improves upon original
- [ ] Documentation is updated
- [ ] No regressions in functionality
