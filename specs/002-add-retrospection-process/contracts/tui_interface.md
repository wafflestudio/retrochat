# TUI Interface Contract: Retrospection Features

## Screen Layout Integration

### Main Session List Panel
**Location**: Primary content area when in session list mode

**New Keyboard Shortcuts**:
- `r`: Trigger retrospection analysis for highlighted session
- `R`: Trigger retrospection analysis for all sessions
- `Ctrl+r`: Show retrospection status panel
- `Ctrl+R`: Cancel all active retrospection operations

**Visual Indicators**:
- `[A]` suffix: Analysis available for this session
- `[P]` suffix: Analysis in progress for this session
- `[F]` suffix: Analysis failed for this session
- Color coding: Green (completed), Blue (running), Red (failed), Gray (not analyzed)

**Example Display**:
```
┌─ Chat Sessions ─────────────────────────────────────────────────┐
│ > claude-coding-2025-09-26-morning.json      [A] ░░░░░░░░░░░    │
│   chatgpt-debugging-2025-09-25.json          [P] ████████░░    │
│   copilot-refactor-2025-09-24.json           [F] ××××××××××    │
│   gemini-analysis-2025-09-23.json                ░░░░░░░░░░░    │
│                                                                │
│ Press 'r' to analyze session, 'R' for all, Ctrl+r for status  │
└────────────────────────────────────────────────────────────────┘
```

### Session Detail Side Panel
**Location**: Right panel when viewing session details

**New Sections**:
- **Analysis Results**: Display retrospection analysis for current session
- **Analysis History**: Show historical analysis results
- **Quick Actions**: Buttons for triggering new analysis

**Layout**:
```
┌─ Session Details ───────────────────────┐
│ Session: claude-coding-morning          │
│ Date: 2025-09-26 09:00:00              │
│ Messages: 45 | Duration: 2h 15m        │
│ ├─ Messages ─────────────────────────── │
│ │ [Message content area]                │
│ │                                       │
│ ├─ Analysis Results ───────────────────  │
│ │ Type: User Interaction Analysis       │
│ │ Date: 2025-09-26 10:30:00            │
│ │ Score: 8/10 | Tokens: 2,847          │
│ │                                       │
│ │ Key Insights:                         │
│ │ • Clear communication style           │
│ │ • Effective problem breakdown         │
│ │ • Good use of examples                │
│ │                                       │
│ │ [Press 'Enter' to view full analysis] │
│ ├─ Quick Actions ─────────────────────── │
│ │ [a] Analyze Session                   │
│ │ [h] View Analysis History             │
│ │ [e] Export Analysis                   │
│ └───────────────────────────────────────┘
```

### Retrospection Management Panel
**Access**: Via Ctrl+r from any screen or dedicated menu option

**Sections**:
1. **Active Operations**: Currently running analysis operations
2. **Operation History**: Recently completed/failed operations
3. **Quick Start**: Common analysis operations
4. **Settings**: Configuration options

**Layout**:
```
┌─ Retrospection Management ──────────────────────────────────────┐
│ ┌─ Active Operations ─────────────────────────────────────────┐ │
│ │ retro-456 | User Interaction | [████████░░] 80% | 2/3      │ │
│ │ retro-789 | Collaboration    | [░░░░░░░░░░] 0%  | 0/5      │ │
│ │                                                             │ │
│ │ [c] Cancel Selected [C] Cancel All [r] Refresh              │ │
│ └─────────────────────────────────────────────────────────────┘ │
│ ┌─ Quick Start ───────────────────────────────────────────────┐ │
│ │ [1] Analyze All Sessions - User Interaction                │ │
│ │ [2] Analyze All Sessions - Collaboration                   │ │
│ │ [3] Analyze Recent Sessions (last 7 days)                  │ │
│ │ [4] Custom Analysis with Prompt                            │ │
│ └─────────────────────────────────────────────────────────────┘ │
│ ┌─ Operation History ─────────────────────────────────────────┐ │
│ │ retro-123 | Task Breakdown   | ✓ Completed | 09:45        │ │
│ │ retro-321 | Question Quality | ✗ Failed    | 09:30        │ │
│ │ retro-654 | Custom Analysis  | ⚪ Cancelled | 09:15        │ │
│ │                                                             │ │
│ │ [↑/↓] Navigate [Enter] View Details [d] Delete             │ │
│ └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Progress Display Components

### Progress Indicator Widget
**Component**: Real-time progress bar for active operations

**Features**:
- Animated progress bar with percentage
- Operation name and current step description
- Estimated time remaining
- Cancel button/keyboard shortcut
- Error state display

**Visual States**:
```
┌─ Analysis Progress ─────────────────────────────────────────────┐
│ Analyzing Session 2 of 5: claude-debugging-session            │
│ [████████████████████░░░░] 80% (4.2/5.0 MB processed)         │
│ Sending request to Google AI... ETA: 45s                      │
│                                                                │
│ [ESC] Cancel Operation [Space] Pause [Enter] View Details     │
└────────────────────────────────────────────────────────────────┘

# Error State
┌─ Analysis Error ───────────────────────────────────────────────┐
│ Failed to analyze session: claude-debugging-session           │
│ Error: Google AI API rate limit exceeded                      │
│ Next retry in: 42 seconds                                     │
│                                                                │
│ [r] Retry Now [s] Skip Session [c] Cancel Operation           │
└────────────────────────────────────────────────────────────────┘
```

### Status Bar Integration
**Location**: Application status bar (bottom of screen)

**Information Displayed**:
- Number of active retrospection operations
- Quick status of most recent operation
- Keyboard shortcut hints

**Examples**:
```
# Normal operation
Retrospection: 2 active | Last: User Analysis ✓ | Ctrl+r for details

# No active operations
Retrospection: idle | Press 'r' on session to analyze | Ctrl+r for history

# Error state
Retrospection: 1 failed | API error - check connection | Ctrl+r for details
```

## Widget Behavior Specifications

### Session List Interactions

**Single Session Analysis**:
1. User highlights session and presses 'r'
2. If session already has analysis: show confirmation dialog
3. Analysis type selection dialog appears
4. Progress indicator shows in place of session item
5. On completion: visual indicator updates, notification shown

**Batch Analysis**:
1. User presses 'R' for all sessions
2. Confirmation dialog with scope details
3. Analysis type selection
4. Background operation starts
5. Global progress indicator appears
6. Sessions update individually as analysis completes

### Session Detail Panel Interactions

**View Analysis Results**:
1. User navigates to session with existing analysis
2. Analysis section automatically populates
3. User can press Enter to view full analysis in popup
4. Navigation between multiple analysis results if available

**Trigger New Analysis**:
1. User presses 'a' in session detail panel
2. Analysis type selection dialog
3. Confirmation with token usage estimate
4. Progress shown in dedicated area
5. Results populate in real-time

### Progress Management

**Operation Cancellation**:
1. User presses ESC or 'c' on active operation
2. Graceful cancellation attempted (30s timeout)
3. Force cancellation option if graceful fails
4. Operation marked as cancelled in history

**Background Operation Monitoring**:
1. Operations continue when user navigates away
2. Status updates propagated to appropriate UI elements
3. Completion notifications shown regardless of current screen
4. Failed operations highlighted for user attention

## Keyboard Shortcuts Summary

### Global Shortcuts
- `Ctrl+r`: Open retrospection management panel
- `Ctrl+R`: Cancel all active operations
- `F9`: Toggle retrospection status bar

### Session List Mode
- `r`: Analyze highlighted session
- `R`: Analyze all sessions
- `Ctrl+a`: Analyze all unanalyzed sessions

### Session Detail Mode
- `a`: Analyze current session
- `h`: View analysis history for session
- `e`: Export analysis results
- `Tab`: Navigate between analysis results

### Retrospection Management Mode
- `1-9`: Quick start operations
- `c`: Cancel selected operation
- `C`: Cancel all operations
- `r`: Refresh operation status
- `d`: Delete operation from history
- `Enter`: View operation details

## Error Handling and User Feedback

### Network Errors
- Clear indication of connectivity issues
- Automatic retry with backoff
- Option to queue operations for later
- Graceful degradation to offline mode

### API Errors
- Specific error messages for common API issues
- Rate limiting awareness with retry timers
- Token usage warnings and cost estimates
- Alternative analysis options when API unavailable

### User Input Validation
- Session selection validation
- Analysis type compatibility checking
- Custom prompt validation
- Resource availability checks before starting operations

### Recovery Actions
- Retry individual failed operations
- Resume interrupted analysis sessions
- Export partial results from failed operations
- Clear failed operations from queue

## Accessibility Features

### Visual Indicators
- Color-blind friendly status indicators
- Text-based status descriptions
- High contrast mode support
- Scalable UI elements

### Keyboard Navigation
- Full keyboard accessibility for all functions
- Consistent navigation patterns
- Keyboard shortcuts documented in help
- Alt-text for visual elements

### Screen Reader Support
- Descriptive labels for all UI elements
- Progress announcements
- Status change notifications
- Structured content hierarchy