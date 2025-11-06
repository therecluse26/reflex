# Interactive Mode Project Plan

**Status:** 🟡 Planning
**Priority:** High
**Estimated Effort:** ~20 hours
**Last Updated:** 2025-11-06

---

## Table of Contents

1. [Overview](#overview)
2. [Goals](#goals)
3. [Design Decisions](#design-decisions)
4. [Features](#features)
5. [Implementation Tasks](#implementation-tasks)
6. [Technical Architecture](#technical-architecture)
7. [Testing Strategy](#testing-strategy)
8. [Future Enhancements](#future-enhancements)

---

## Overview

**Interactive Mode** transforms Reflex into a live, exploratory code search interface. Instead of running one-off queries, users can interactively search, filter, navigate, and explore code results in a modern terminal UI.

### Vision

Make code search as intuitive as using `fzf` or `lazygit` - launch with a single command, type to search, see instant results, and navigate naturally with keyboard shortcuts.

### Why This Matters

**Current workflow (CLI):**
```bash
rfx query "extract_symbols"          # Run query
# See results
rfx query "extract_symbols" --symbols  # Refine
# See different results
rfx query "extract_symbols" --lang rust  # Refine again
```

**New workflow (Interactive):**
```bash
rfx                                  # Launch interactive mode
# Type "extract_symbols" → see results
# Press 's' → toggle symbols mode
# Press 'l' → filter to Rust
# Click result → opens in editor
```

**Benefits:**
- **Faster iteration**: No process spawning between queries
- **Better discoverability**: See filters and shortcuts in UI
- **Lower friction**: One command to explore entire codebase
- **Modern UX**: Matches expectations set by modern TUI tools

---

## Goals

### Primary Goals

1. ✅ **Make Reflex default to interactive mode**
   - Running `rfx` with no arguments launches TUI
   - Reduces friction for exploratory workflows

2. ✅ **Provide instant feedback**
   - Live search with <300ms latency
   - Show results as user types (debounced)

3. ✅ **Integrate seamlessly with editors**
   - Clickable file paths (OSC 8 hyperlinks)
   - Fallback `o` key for all terminals

4. ✅ **Maintain Reflex's core principles**
   - Deterministic: same query = same results
   - Fast: leverage existing query engine
   - Local-first: no network, no daemon

### Success Criteria

**Must achieve:**
- [ ] Launch `rfx` → interactive mode appears in <100ms
- [ ] Type query → see results in <300ms (debounced)
- [ ] Navigate 500+ results without UI lag
- [ ] Click file path → opens in editor (on supported terminals)
- [ ] Press `o` → opens in `$EDITOR` (universal fallback)
- [ ] Press `i` → triggers reindex
- [ ] Ctrl+P → recalls previous query with filters
- [ ] Press `?` → see comprehensive help

**Should achieve:**
- [ ] Auto-detect stale index and prompt reindex
- [ ] Syntax highlight code previews
- [ ] Show indexing progress with spinner
- [ ] Persist 1000 queries to history

**Nice to have:**
- [ ] Expand result to show full function body
- [ ] Mouse support for clicking results
- [ ] Export results to JSON from TUI

---

## Design Decisions

### 1. Default Command Behavior

**Decision:** `rfx` (no arguments) launches interactive mode

**Rationale:**
- Matches modern TUI tools (`lazygit`, `gitui`, `bottom`)
- Reduces typing for primary use case
- Other commands remain explicit: `rfx index`, `rfx serve`, etc.

**Implementation:**
```rust
// src/cli.rs
if no subcommand provided {
    handle_interactive()?;
}
```

---

### 2. No CLI Options for Interactive Mode

**Decision:** No `--path`, `--lang`, or `--plain` flags

**Rationale:**
- Simpler UX (fewer decisions upfront)
- Path: user can `cd` to directory first
- Lang: set via filter menu inside TUI (more discoverable)
- Plain: auto-detect terminal capabilities

**Rejected alternative:** Adding options adds complexity without clear benefit.

---

### 3. Clickable File Links (OSC 8 Hyperlinks)

**Decision:** Use OSC 8 hyperlinks when terminal supports it, fall back to `o` key

**Rationale:**
- Modern terminals (iTerm2, WezTerm, Kitty, VSCode) support OSC 8
- Provides native "click to open" UX
- Graceful degradation: `o` key works everywhere

**Terminal Detection:**
```rust
fn supports_hyperlinks() -> bool {
    match std::env::var("TERM_PROGRAM") {
        Ok(term) => matches!(term.as_str(), "iTerm.app" | "WezTerm" | "vscode"),
        Err(_) => std::env::var("TERM").map(|t| t.contains("kitty")).unwrap_or(false),
    }
}
```

**Adaptive UI:**
- **With hyperlinks:** Show `[Cmd+Click to open]` hint
- **Without hyperlinks:** Show `[Press 'o' to open]` hint

---

### 4. Query History Persistence

**Decision:** Store only queries (not results) in JSON format

**Rationale:**
- Queries are deterministic: same query = same results
- Results regenerated from latest index (always fresh)
- Tiny file size (<10KB for 1000 queries)

**Format:**
```json
{
  "queries": [
    {
      "pattern": "extract_symbols",
      "timestamp": "2025-11-06T12:34:56Z",
      "filters": {
        "symbols_mode": true,
        "language": "Rust",
        "kind": "Function"
      }
    }
  ],
  "max_entries": 1000
}
```

**Location:** `~/.reflex/interactive_history.json`

---

### 5. Result Limits

**Decision:** Show up to 500 results, no pagination

**Rationale:**
- 500 results render instantly in ratatui
- Encourages query refinement (better UX than infinite scroll)
- Prevents UI lag

**Behavior:**
- If query returns >500 results, show first 500
- Display message: `"Showing first 500 results. Refine query for precision."`

---

### 6. Syntax Highlighting Theme

**Decision:** Auto-detect terminal background (dark vs light)

**Rationale:**
- No user configuration needed
- Works correctly 90% of the time
- Can be overridden in `~/.reflex/config.toml` later

**Implementation:**
```rust
fn detect_theme() -> &'static str {
    // Parse COLORFGBG environment variable
    // Format: "foreground;background" where 0-7=dark, 8-15=light
    if let Ok(colorfgbg) = std::env::var("COLORFGBG") {
        if let Some(bg) = colorfgbg.split(';').nth(1) {
            if let Ok(bg_val) = bg.parse::<u8>() {
                return if bg_val < 8 { "Monokai" } else { "InspiredGitHub" };
            }
        }
    }
    "Monokai" // Default to dark
}
```

**Themes:**
- Dark: `Monokai` (high contrast, popular)
- Light: `InspiredGitHub` (clean, readable)

---

### 7. Auto-Indexing on Startup

**Decision:** Automatically index if cache is missing or stale

**Rationale:**
- Ensures search results are always fresh
- Eliminates manual `rfx index` step
- Shows progress so user knows what's happening

**Stale Detection:**
- Check if any tracked files modified since last index
- Compare current file hashes with cached hashes
- Warn: `[Index: ⚠️ Stale (3 files changed) - Press 'i' to reindex]`

**UI:**
```
┌─────────────────────────── Reflex Interactive ─────────────────────────────┐
│ ⏳ Indexing...  [████████░░░░░░░░░░░░] 234/567 files  (src/main.rs)       │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Features

### Phase 1: MVP (Core Functionality)

All features below are **required** for Phase 1.

#### F1.1: Default Command Launch
- **Description:** `rfx` with no arguments launches interactive mode
- **Acceptance Criteria:**
  - Running `rfx` displays TUI within <100ms
  - Existing commands still work: `rfx index`, `rfx query`, etc.
- **Estimated Time:** 1 hour

#### F1.2: Help Screen on First Launch
- **Description:** Show welcome screen with keyboard shortcuts
- **Acceptance Criteria:**
  - First-time users see help screen automatically
  - Help screen lists all keyboard shortcuts
  - `?` key toggles help at any time
- **Estimated Time:** 1 hour

#### F1.3: Auto-Index with Progress
- **Description:** Automatically index if cache missing/stale, show progress
- **Acceptance Criteria:**
  - Detects missing `.reflex/` cache
  - Detects stale cache (files changed since last index)
  - Shows indexing progress: `[████░░░░] 234/567 files`
  - Displays current file being indexed
  - Updates UI in real-time
- **Estimated Time:** 2 hours

#### F1.4: Compact Index Status Indicator
- **Description:** Always-visible status in top-right corner
- **Acceptance Criteria:**
  - Shows: `[Index: ✓ 567 files, 2s ago]`
  - Updates when reindex triggered
  - Shows stale warning: `[Index: ⚠️ Stale (3 changes)]`
- **Estimated Time:** 30 minutes

#### F1.5: Query Input Box
- **Description:** Text input for search pattern
- **Acceptance Criteria:**
  - Cursor visible and blinking
  - Supports standard text editing (backspace, arrow keys)
  - Shows character count (optional)
- **Estimated Time:** 1 hour

#### F1.6: Live Search with Debouncing
- **Description:** Execute search as user types (300ms debounce)
- **Acceptance Criteria:**
  - Waits 300ms after last keystroke before searching
  - Shows loading indicator during search
  - Displays result count after search completes
- **Estimated Time:** 2 hours

#### F1.7: Result List Display
- **Description:** Scrollable list of search results
- **Acceptance Criteria:**
  - Shows up to 500 results
  - Displays file path, line number, and context
  - Highlights selected result
  - Shows scroll position indicator
- **Estimated Time:** 2 hours

#### F1.8: Code Preview with Syntax Highlighting
- **Description:** Show code snippet with 5 lines of context
- **Acceptance Criteria:**
  - Syntax highlighting based on file extension
  - Auto-detect terminal background (dark/light)
  - Highlight matching pattern
  - Show line numbers
- **Estimated Time:** 3 hours

#### F1.9: OSC 8 Clickable Links
- **Description:** Make file paths clickable (when supported)
- **Acceptance Criteria:**
  - Detect terminal support for OSC 8
  - Generate hyperlinks: `file:///path/to/file:123`
  - Show `[Cmd+Click to open]` hint when supported
  - Show `[Press 'o' to open]` hint otherwise
- **Estimated Time:** 1 hour

#### F1.10: Keyboard Navigation
- **Description:** Navigate results with keyboard
- **Keyboard Shortcuts:**
  - `j` / `↓` - Next result
  - `k` / `↑` - Previous result
  - `Page Down` - Jump 10 results down
  - `Page Up` - Jump 10 results up
  - `Home` - First result
  - `End` - Last result
  - `/` - Focus search input
  - `Esc` - Unfocus input / cancel
- **Estimated Time:** 1 hour

#### F1.11: Filter Toggles
- **Description:** Toggle search modes and filters
- **Keyboard Shortcuts:**
  - `s` - Toggle symbols-only mode
  - `r` - Toggle regex mode
  - `l` - Prompt for language filter (inline)
  - `k` - Prompt for symbol kind filter (inline)
- **Acceptance Criteria:**
  - Filters update query in real-time
  - Display active filters as badges in header
  - Clear/reset filters with `Esc`
- **Estimated Time:** 2 hours

#### F1.12: Open in Editor
- **Description:** Open selected result in `$EDITOR`
- **Keyboard Shortcuts:**
  - `o` - Open file at line number
- **Acceptance Criteria:**
  - Detects `$EDITOR` environment variable
  - Falls back to `vim` if unset
  - Passes line number to editor (e.g., `vim +123 file.rs`)
  - Supports: vim, nvim, emacs, vscode, nano
- **Estimated Time:** 1 hour

#### F1.13: Query History
- **Description:** Persist and recall previous queries
- **Keyboard Shortcuts:**
  - `Ctrl+P` - Previous query
  - `Ctrl+N` - Next query
- **Acceptance Criteria:**
  - Saves to `~/.reflex/interactive_history.json`
  - Stores last 1000 queries
  - Restores query pattern + filters
  - Deduplicates identical queries
- **Estimated Time:** 2 hours

#### F1.14: Manual Reindex
- **Description:** Trigger reindex from TUI
- **Keyboard Shortcuts:**
  - `i` - Start reindex
- **Acceptance Criteria:**
  - Shows indexing progress
  - Updates status indicator when complete
  - Automatically re-runs current query
- **Estimated Time:** 1 hour

#### F1.15: Quit
- **Description:** Exit interactive mode cleanly
- **Keyboard Shortcuts:**
  - `q` - Quit
  - `Ctrl+C` - Quit
- **Acceptance Criteria:**
  - Restores terminal state
  - Saves history before exit
- **Estimated Time:** 30 minutes

---

### Phase 2: Enhanced Features (Future)

#### F2.1: Result Expansion
- **Description:** Show full function/class body
- **Keyboard Shortcuts:**
  - `Enter` - Expand/collapse selected result
- **Estimated Time:** 2 hours

#### F2.2: Advanced Filter Menu
- **Description:** Full-screen filter editor
- **Keyboard Shortcuts:**
  - `f` - Open filter menu
- **Features:**
  - File pattern (glob)
  - Exact match toggle
  - Result limit
  - Multi-language selection
- **Estimated Time:** 3 hours

#### F2.3: Mouse Support
- **Description:** Click to select results, scroll with wheel
- **Estimated Time:** 2 hours

#### F2.4: Split View
- **Description:** Side-by-side results + full file preview
- **Estimated Time:** 4 hours

#### F2.5: Export Results
- **Description:** Export current results to JSON
- **Keyboard Shortcuts:**
  - `e` - Export to file
- **Estimated Time:** 1 hour

---

## Implementation Tasks

### Task Breakdown (Phase 1)

| # | Task | Description | Time | Status |
|---|------|-------------|------|--------|
| 1 | Project structure | Create `src/interactive/` module with files | 30m | ⬜ Todo |
| 2 | CLI integration | Make `rfx` default to interactive mode | 1h | ⬜ Todo |
| 3 | Terminal detection | Detect OSC 8 support, background color | 1h | ⬜ Todo |
| 4 | App state | Define `InteractiveApp` struct | 1h | ⬜ Todo |
| 5 | Index status & auto-index | Check cache, auto-index if stale | 2h | ⬜ Todo |
| 6 | Basic TUI layout | Top bar, results area, help bar | 2h | ⬜ Todo |
| 7 | Query input & search | Text input + debounced search | 2h | ⬜ Todo |
| 8 | Result display | List + code preview + syntax highlighting | 3h | ⬜ Todo |
| 9 | Navigation | Keyboard shortcuts for scrolling | 1h | ⬜ Todo |
| 10 | Filters | Symbols, regex, language, kind toggles | 2h | ⬜ Todo |
| 11 | File opening | Open in `$EDITOR` with line number | 1h | ⬜ Todo |
| 12 | Query history | JSON persistence + Ctrl+P/N navigation | 2h | ⬜ Todo |
| 13 | Help screen | Show shortcuts, display on first launch | 1h | ⬜ Todo |
| 14 | Polish & error handling | Edge cases, loading states, cleanup | 2h | ⬜ Todo |

**Total Estimated Time:** ~20 hours

---

## Technical Architecture

### Module Structure

```
src/interactive/
├── mod.rs           # Public API: run_interactive()
├── app.rs           # Application state and event loop
├── ui.rs            # TUI rendering with ratatui
├── input.rs         # Input handling and command parsing
├── results.rs       # Result list management and navigation
├── history.rs       # Query history persistence (JSON)
├── terminal.rs      # Terminal capability detection
└── theme.rs         # Syntax theme auto-detection
```

---

### Data Structures

#### `InteractiveApp` (src/interactive/app.rs)

```rust
pub struct InteractiveApp {
    // Query state
    query: String,
    cursor_pos: usize,

    // Results (regenerated on each query)
    results: Vec<SearchResult>,
    selected_index: usize,
    scroll_offset: usize,

    // Filters
    filter: QueryFilter,

    // History (persisted to disk)
    history: QueryHistory,
    history_cursor: Option<usize>,

    // Index status
    index_status: IndexStatus,
    indexing_progress: Option<IndexingProgress>,

    // UI state
    mode: AppMode,  // Input, Results, FilterPrompt, Help

    // Engine (reused from existing code)
    engine: QueryEngine,

    // Terminal capabilities
    supports_hyperlinks: bool,
    terminal_type: TerminalType,
    theme: Theme,
}

pub enum AppMode {
    Input,         // Editing query
    Results,       // Browsing results
    FilterPrompt,  // Entering filter value (lang, kind)
    Help,          // Showing help screen
}

pub enum IndexStatus {
    Ready { files: usize, last_updated: String },
    Indexing { progress: IndexingProgress },
    Stale { files_changed: usize },
    Missing,
}

pub struct IndexingProgress {
    current: usize,
    total: usize,
    current_file: String,
}
```

#### `QueryHistory` (src/interactive/history.rs)

```rust
pub struct QueryHistory {
    queries: VecDeque<HistoricalQuery>,
    max_size: usize,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct HistoricalQuery {
    pattern: String,
    timestamp: String,
    filters: QueryFilter,
}

impl QueryHistory {
    pub fn load() -> Result<Self>;
    pub fn save(&self) -> Result<()>;
    pub fn add(&mut self, query: HistoricalQuery);
    pub fn prev(&mut self) -> Option<&HistoricalQuery>;
    pub fn next(&mut self) -> Option<&HistoricalQuery>;
}
```

---

### Event Loop

```rust
pub fn run_interactive() -> Result<()> {
    let mut terminal = setup_terminal()?;
    let mut app = InteractiveApp::new()?;

    // Auto-index if needed
    if app.index_status.needs_indexing() {
        app.start_indexing()?;
    }

    loop {
        // Render UI
        terminal.draw(|f| ui::render(f, &app))?;

        // Handle events
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if !app.handle_key(key)? {
                        break; // User quit
                    }
                }
                Event::Resize(_, _) => {
                    // Terminal resized, re-render
                }
                _ => {}
            }
        }

        // Execute search if query changed
        if app.should_search() {
            app.execute_search()?;
        }

        // Update indexing progress
        if app.is_indexing() {
            app.update_indexing_progress()?;
        }
    }

    restore_terminal(terminal)?;
    Ok(())
}
```

---

### UI Layout (Ratatui)

```rust
pub fn render(f: &mut Frame, app: &InteractiveApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header (query + status)
            Constraint::Min(1),     // Results area
            Constraint::Length(1),  // Help bar
        ])
        .split(f.size());

    render_header(f, chunks[0], app);
    render_results(f, chunks[1], app);
    render_help_bar(f, chunks[2], app);
}

fn render_header(f: &mut Frame, area: Rect, app: &InteractiveApp) {
    // Query input box + index status
}

fn render_results(f: &mut Frame, area: Rect, app: &InteractiveApp) {
    if app.mode == AppMode::Help {
        render_help_screen(f, area, app);
    } else {
        render_result_list(f, area, app);
    }
}

fn render_help_bar(f: &mut Frame, area: Rect, app: &InteractiveApp) {
    // Keyboard shortcuts
}
```

---

## Testing Strategy

### Unit Tests

**Test coverage:**
- `QueryHistory` load/save/navigate
- Terminal detection (`supports_hyperlinks()`)
- Theme auto-detection (`detect_theme()`)
- OSC 8 link generation
- Debounce logic

**Example:**
```rust
#[test]
fn test_query_history_deduplication() {
    let mut history = QueryHistory::new(100);

    let query1 = HistoricalQuery {
        pattern: "test".to_string(),
        filters: QueryFilter::default(),
        timestamp: "2025-11-06T12:00:00Z".to_string(),
    };

    history.add(query1.clone());
    history.add(query1.clone()); // Duplicate

    assert_eq!(history.len(), 1); // Should deduplicate
}
```

---

### Integration Tests

**Manual testing checklist:**

#### Terminal Compatibility
- [ ] iTerm2 (macOS) - Hyperlinks work
- [ ] WezTerm - Hyperlinks work
- [ ] Kitty - Hyperlinks work
- [ ] VSCode terminal - Hyperlinks work
- [ ] Alacritty - Falls back to `o` key
- [ ] tmux - Works inside tmux session
- [ ] GNU Screen - Works inside screen session

#### Functionality
- [ ] Launch `rfx` → TUI appears
- [ ] Type query → results appear
- [ ] Navigate with j/k → selection moves
- [ ] Press `s` → toggles symbols mode
- [ ] Press `l` → prompts for language
- [ ] Press `o` → opens in editor
- [ ] Click file path → opens in editor (if supported)
- [ ] Press `i` → triggers reindex
- [ ] Ctrl+P → recalls previous query
- [ ] Press `?` → shows help
- [ ] Press `q` → quits cleanly

#### Edge Cases
- [ ] Empty query → no results
- [ ] Query with 0 results → helpful message
- [ ] Query with >500 results → shows first 500
- [ ] Very long file path → truncates gracefully
- [ ] Very long query → scrolls input box
- [ ] Terminal resize → redraws correctly
- [ ] Ctrl+C during indexing → cancels gracefully

---

### Performance Tests

**Benchmarks:**
- [ ] Launch time: <100ms from command to TUI
- [ ] Query latency: <300ms for debounced search
- [ ] Render time: <16ms per frame (60 FPS)
- [ ] Scroll performance: No lag with 500 results
- [ ] History load: <50ms for 1000 entries

---

## Future Enhancements

### Phase 2: Enhanced Features

#### Mouse Support
- Click to select results
- Scroll wheel for navigation
- Estimated: 2 hours

#### Result Expansion
- Press `Enter` to show full function/class body
- Collapsible sections
- Estimated: 2 hours

#### Advanced Filter Menu
- Full-screen filter editor
- File pattern (glob)
- Multi-language selection
- Exact match toggle
- Estimated: 3 hours

#### Split View
- Side-by-side: results + full file preview
- Toggle with `Tab`
- Estimated: 4 hours

#### Export Results
- Press `e` to export to JSON
- Estimated: 1 hour

---

### Phase 3: Power User Features

#### Search History Suggestions
- Autocomplete from previous queries
- Fuzzy match on history
- Estimated: 3 hours

#### Result Bookmarks
- Press `b` to bookmark result
- View bookmarks with `B`
- Persist to `~/.reflex/bookmarks.json`
- Estimated: 2 hours

#### Multi-Query Mode
- Open multiple queries in tabs
- Switch with `1`, `2`, `3` keys
- Estimated: 4 hours

#### Custom Key Bindings
- User-configurable in `~/.reflex/config.toml`
- Vim-style or Emacs-style presets
- Estimated: 3 hours

---

## Open Questions

### Resolved
- ✅ Should `rfx` default to interactive mode? **YES**
- ✅ Should we support CLI options for interactive mode? **NO**
- ✅ How many results to display? **500 max, no pagination**
- ✅ How many queries to store in history? **1000**
- ✅ When to show clickable links? **Only when terminal supports OSC 8**
- ✅ Which syntax theme to use? **Auto-detect terminal background**

### Pending
- ⏳ Should first launch show help or empty query? **Help screen**
- ⏳ Auto-index on startup even if cache exists but is stale? **YES**
- ⏳ Should we show indexing progress in the background while user types? **TBD**
- ⏳ How to handle Ctrl+C during indexing? **Cancel and show partial results**

---

## References

### External Documentation
- [Ratatui Documentation](https://ratatui.rs/)
- [Crossterm Documentation](https://docs.rs/crossterm/)
- [OSC 8 Hyperlinks Spec](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda)
- [Syntect Documentation](https://docs.rs/syntect/)

### Internal Documentation
- [ARCHITECTURE.md](ARCHITECTURE.md) - Reflex system design
- [CLAUDE.md](../CLAUDE.md) - Project overview and workflow

---

## Change Log

### 2025-11-06
- Initial project plan created
- Defined goals, features, and implementation tasks
- Estimated ~20 hours for Phase 1 MVP
