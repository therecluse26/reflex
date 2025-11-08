use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent, MouseEvent};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use crate::cache::CacheManager;
use crate::indexer::Indexer;
use crate::models::{IndexConfig, SearchResult};
use crate::query::{QueryEngine, QueryFilter};

use super::effects::EffectManager;
use super::history::{QueryFilters, QueryHistory};
use super::input::{InputField, KeyCommand};
use super::mouse::{MouseAction, MouseState};
use super::results::ResultList;
use super::terminal::TerminalCapabilities;
use super::theme::ThemeManager;
use super::ui;

/// Main application state for interactive mode
pub struct InteractiveApp {
    /// Query input field
    input: InputField,
    /// Search results
    results: ResultList,
    /// Query history
    history: QueryHistory,
    /// Current filter state
    filters: QueryFilters,
    /// Query engine
    engine: QueryEngine,
    /// Cache manager
    cache: CacheManager,
    /// Current application mode
    mode: AppMode,
    /// Terminal capabilities
    capabilities: TerminalCapabilities,
    /// Theme manager
    theme: ThemeManager,
    /// Effect manager for animations
    effects: EffectManager,
    /// Mouse state
    mouse: MouseState,
    /// Whether a search is pending (debounce)
    search_pending: bool,
    /// Last search time (for debouncing)
    last_input_time: Option<Instant>,
    /// Debounce duration in milliseconds
    debounce_ms: u64,
    /// Index status
    index_status: IndexStatusState,
    /// Whether to quit
    should_quit: bool,
    /// Current focus state
    focus_state: FocusState,
    /// Current working directory
    cwd: PathBuf,
    /// Error message to display (if any)
    error_message: Option<String>,
    /// Info message to display (if any)
    info_message: Option<String>,
    /// File preview content (when a result is expanded)
    preview_content: Option<FilePreview>,
    /// Whether a search is currently executing
    searching: bool,
    /// Channel receiver for async search results
    search_rx: Option<mpsc::Receiver<Result<crate::models::QueryResponse>>>,
    /// Whether indexing is currently in progress
    indexing: bool,
    /// Channel receiver for async indexing results
    index_rx: Option<mpsc::Receiver<Result<crate::models::IndexStats>>>,
    /// Channel receiver for indexing progress updates (current, total, status)
    index_progress_rx: Option<mpsc::Receiver<(usize, usize, String)>>,
    /// Indexing start time (for elapsed time display)
    indexing_start_time: Option<Instant>,
}

/// File preview state
#[derive(Debug, Clone)]
pub struct FilePreview {
    path: String,
    content: Vec<String>,
    center_line: usize,
    scroll_offset: usize,
}

/// Application mode
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// Normal browsing mode
    Normal,
    /// Help screen is showing
    Help,
    /// Indexing in progress
    Indexing,
    /// File preview is showing
    FilePreview,
}

/// Focus state for Tab navigation
#[derive(Debug, Clone, PartialEq)]
pub enum FocusState {
    Input,
    Filters,
    Results,
}

/// Index status state
#[derive(Debug, Clone)]
pub enum IndexStatusState {
    /// Index is ready
    Ready {
        file_count: usize,
        last_updated: String,
    },
    /// Index doesn't exist
    Missing,
    /// Index is stale (files changed)
    Stale {
        files_changed: usize,
    },
    /// Currently indexing
    Indexing {
        current: usize,
        total: usize,
        status: String,
    },
}

impl InteractiveApp {
    /// Create a new interactive application
    pub fn new() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        let cache = CacheManager::new(&cwd);
        let cache2 = CacheManager::new(&cwd); // Create second instance for engine
        let engine = QueryEngine::new(cache2);
        let capabilities = TerminalCapabilities::detect();
        let theme = ThemeManager::detect();
        let history = QueryHistory::load().unwrap_or_else(|_| QueryHistory::new(1000));

        // Check index status
        let index_status = if cache.exists() {
            // Get actual file count from cache stats
            match cache.stats() {
                Ok(stats) => IndexStatusState::Ready {
                    file_count: stats.total_files,
                    last_updated: stats.last_updated,
                },
                Err(_) => IndexStatusState::Missing,
            }
        } else {
            IndexStatusState::Missing
        };

        Ok(Self {
            input: InputField::new(),
            results: ResultList::new(500),
            history,
            filters: QueryFilters::default(),
            engine,
            cache,
            mode: AppMode::Normal,
            capabilities,
            theme,
            effects: EffectManager::new(),
            mouse: MouseState::new(),
            search_pending: false,
            last_input_time: None,
            debounce_ms: 600, // 2x longer debounce for smoother typing
            index_status,
            should_quit: false,
            focus_state: FocusState::Input, // Start with input focused
            cwd,
            error_message: None,
            info_message: None,
            preview_content: None,
            searching: false,
            search_rx: None,
            indexing: false,
            index_rx: None,
            index_progress_rx: None,
            indexing_start_time: None,
        })
    }

    /// Run the interactive event loop
    pub fn run(&mut self) -> Result<()> {
        // Show help on first launch (if history is empty)
        if self.history.is_empty() {
            self.mode = AppMode::Help;
        }

        // Setup terminal FIRST
        let mut terminal = Self::setup_terminal()?;

        // Check if we need to trigger indexing (will show modal in event loop)
        let needs_index = matches!(self.index_status, IndexStatusState::Missing);

        // Main event loop (will handle deferred indexing with modal)
        let result = self.event_loop(&mut terminal, needs_index);

        // Restore terminal
        Self::restore_terminal(terminal)?;

        // Save history on exit
        if let Err(e) = self.history.save() {
            eprintln!("Warning: Failed to save history: {}", e);
        }

        result
    }

    fn event_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, needs_index: bool) -> Result<()> {
        let mut last_frame = Instant::now();
        let frame_duration = Duration::from_millis(16); // ~60 FPS
        let mut need_editor_open: Option<SearchResult> = None;
        let mut first_frame = true;

        while !self.should_quit {
            // Trigger indexing on first frame if needed (UI loads first, then shows modal)
            if first_frame && needs_index {
                self.trigger_index()?;
                first_frame = false;
            }

            // Update visible height for scroll calculations
            let terminal_height = terminal.size()?.height;
            let visible_height = terminal_height.saturating_sub(9) as usize; // header(3) + filters(3) + footer(1) + result_borders(2)
            self.results.set_visible_height(visible_height);

            // Render UI
            terminal.draw(|f| ui::render(f, self))?;

            // Handle deferred editor opening (after rendering)
            if let Some(result) = need_editor_open.take() {
                self.open_in_editor_suspended(terminal, &result)?;
            }

            // Handle events (with timeout for smooth rendering)
            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => {
                        if let Some(result) = self.handle_key_event_with_editor(key)? {
                            need_editor_open = Some(result);
                        }
                    }
                    Event::Mouse(mouse) => self.handle_mouse_event(mouse),
                    Event::Resize(_, _) => {
                        // Terminal resized, will redraw on next frame
                    }
                    _ => {}
                }
            }

            // Check if we need to execute a search
            if self.should_execute_search() {
                self.execute_search()?;
            }

            // Check for search results
            if let Some(ref rx) = self.search_rx {
                if let Ok(result) = rx.try_recv() {
                    // Search completed
                    match result {
                        Ok(response) => {
                            self.results.set_results(response.results);
                            self.error_message = None;

                            // Add to history
                            let pattern = self.input.value().to_string();
                            self.history.add(pattern, self.filters.clone());

                            // Auto-move to results after search
                            if !self.results.is_empty() {
                                self.focus_state = FocusState::Results;
                            }
                        }
                        Err(e) => {
                            self.error_message = Some(format!("Search error: {}", e));
                            self.results.clear();
                        }
                    }
                    self.searching = false;
                    self.search_rx = None;
                }
            }

            // Check for indexing progress updates
            if let Some(ref rx) = self.index_progress_rx {
                if let Ok((current, total, status)) = rx.try_recv() {
                    // Update progress state
                    self.index_status = IndexStatusState::Indexing {
                        current,
                        total,
                        status,
                    };
                }
            }

            // Check for indexing results
            if let Some(ref rx) = self.index_rx {
                if let Ok(result) = rx.try_recv() {
                    // Indexing completed
                    match result {
                        Ok(stats) => {
                            self.index_status = IndexStatusState::Ready {
                                file_count: stats.total_files,
                                last_updated: "just now".to_string(),
                            };
                            // Don't re-trigger search - keep current results
                        }
                        Err(e) => {
                            self.error_message = Some(format!("Index error: {}", e));
                        }
                    }
                    self.indexing = false;
                    self.indexing_start_time = None;
                    self.index_rx = None;
                    self.index_progress_rx = None;
                }
            }

            // Update effects
            let elapsed = last_frame.elapsed();
            self.effects.update(elapsed);
            last_frame = Instant::now();

            // Frame pacing
            std::thread::sleep(frame_duration.saturating_sub(elapsed));
        }

        Ok(())
    }

    fn handle_key_event_with_editor(&mut self, key: KeyEvent) -> Result<Option<SearchResult>> {
        // Handle Tab/Shift+Tab for focus cycling
        if key.code == crossterm::event::KeyCode::Tab {
            if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                self.focus_prev();
            } else {
                self.focus_next();
            }
            return Ok(None);
        }

        // Handle Escape - close preview or unfocus
        if key.code == crossterm::event::KeyCode::Esc {
            if self.mode == AppMode::FilePreview {
                self.mode = AppMode::Normal;
                self.preview_content = None;
                return Ok(None);
            }
            self.focus_state = FocusState::Results;
            return Ok(None);
        }

        // Handle Enter - different behavior based on focus
        if key.code == crossterm::event::KeyCode::Enter {
            match self.focus_state {
                FocusState::Input => {
                    // Execute search and move to results
                    if !self.input.value().trim().is_empty() {
                        self.execute_search()?;
                        self.focus_state = FocusState::Results;
                    }
                }
                FocusState::Results => {
                    // Expand file preview
                    if let Some(result) = self.results.selected().cloned() {
                        self.show_file_preview(&result)?;
                    }
                }
                _ => {}
            }
            return Ok(None);
        }

        let command = KeyCommand::from_key(key, self.focus_state == FocusState::Input);

        match command {
            KeyCommand::Quit => {
                self.should_quit = true;
                Ok(None)
            }

            KeyCommand::ShowHelp => {
                self.mode = if self.mode == AppMode::Help {
                    AppMode::Normal
                } else {
                    AppMode::Help
                };
                Ok(None)
            }

            KeyCommand::FocusInput => {
                self.focus_state = FocusState::Input;
                Ok(None)
            }

            KeyCommand::UnfocusInput => {
                self.focus_state = FocusState::Results;
                Ok(None)
            }

            KeyCommand::NextResult => {
                if self.mode == AppMode::FilePreview {
                    self.scroll_preview_down();
                } else {
                    self.results.next();
                }
                Ok(None)
            }

            KeyCommand::PrevResult => {
                if self.mode == AppMode::FilePreview {
                    self.scroll_preview_up();
                } else {
                    self.results.prev();
                }
                Ok(None)
            }

            KeyCommand::PageDown => {
                self.results.jump_down(10);
                Ok(None)
            }

            KeyCommand::PageUp => {
                self.results.jump_up(10);
                Ok(None)
            }

            KeyCommand::First => {
                self.results.first();
                Ok(None)
            }

            KeyCommand::Last => {
                self.results.last();
                Ok(None)
            }

            KeyCommand::ToggleSymbols => {
                self.filters.symbols_mode = !self.filters.symbols_mode;
                self.trigger_search();
                Ok(None)
            }

            KeyCommand::ToggleRegex => {
                self.filters.regex_mode = !self.filters.regex_mode;
                self.trigger_search();
                Ok(None)
            }

            KeyCommand::OpenInEditor => {
                // Return the result to open in editor
                Ok(self.results.selected().cloned())
            }

            KeyCommand::Reindex => {
                self.trigger_index()?;
                Ok(None)
            }

            KeyCommand::HistoryPrev => {
                if let Some(query) = self.history.prev() {
                    self.input.set_value(query.pattern.clone());
                    self.filters = query.filters.clone();
                    self.trigger_search();
                }
                Ok(None)
            }

            KeyCommand::HistoryNext => {
                if let Some(query) = self.history.next() {
                    self.input.set_value(query.pattern.clone());
                    self.filters = query.filters.clone();
                    self.trigger_search();
                } else {
                    // At the end of history, clear input
                    self.input.clear();
                    self.results.clear();
                }
                Ok(None)
            }

            KeyCommand::None => {
                // If input is focused, handle the key for text input
                if self.focus_state == FocusState::Input {
                    if self.input.handle_key(key) {
                        // Input changed, trigger debounced search
                        self.trigger_search();
                    }
                }
                Ok(None)
            }

            _ => Ok(None),
        }
    }

    fn focus_next(&mut self) {
        self.focus_state = match self.focus_state {
            FocusState::Input => FocusState::Filters,
            FocusState::Filters => FocusState::Results,
            FocusState::Results => FocusState::Input,
        };
    }

    fn focus_prev(&mut self) {
        self.focus_state = match self.focus_state {
            FocusState::Input => FocusState::Results,
            FocusState::Filters => FocusState::Input,
            FocusState::Results => FocusState::Filters,
        };
    }

    fn show_file_preview(&mut self, result: &SearchResult) -> Result<()> {
        // Read file content
        let content = std::fs::read_to_string(&result.path)?;
        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

        self.preview_content = Some(FilePreview {
            path: result.path.clone(),
            content: lines,
            center_line: result.span.start_line,
            scroll_offset: result.span.start_line.saturating_sub(10),
        });

        self.mode = AppMode::FilePreview;
        Ok(())
    }

    fn scroll_preview_down(&mut self) {
        if let Some(ref mut preview) = self.preview_content {
            if preview.scroll_offset + 20 < preview.content.len() {
                preview.scroll_offset += 1;
            }
        }
    }

    fn scroll_preview_up(&mut self) {
        if let Some(ref mut preview) = self.preview_content {
            preview.scroll_offset = preview.scroll_offset.saturating_sub(1);
        }
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        // In preview mode, handle scroll events for file content
        if self.mode == AppMode::FilePreview {
            match mouse.kind {
                crossterm::event::MouseEventKind::ScrollDown => {
                    for _ in 0..3 {
                        self.scroll_preview_down();
                    }
                }
                crossterm::event::MouseEventKind::ScrollUp => {
                    for _ in 0..3 {
                        self.scroll_preview_up();
                    }
                }
                _ => {}
            }
            return;
        }

        // In normal mode, handle result selection and scrolling
        let result_area = ratatui::layout::Rect::new(0, 3, 80, 20);
        let action = self.mouse.handle_event(mouse, result_area);

        match action {
            MouseAction::SelectResult(index) => {
                self.results.select(index + self.results.scroll_offset());
            }
            MouseAction::ScrollDown => {
                for _ in 0..3 {
                    self.results.next();
                }
            }
            MouseAction::ScrollUp => {
                for _ in 0..3 {
                    self.results.prev();
                }
            }
            _ => {}
        }
    }

    fn trigger_search(&mut self) {
        self.search_pending = true;
        self.last_input_time = Some(Instant::now());
        self.history.reset_cursor();
    }

    fn should_execute_search(&self) -> bool {
        if !self.search_pending {
            return false;
        }

        if let Some(last_time) = self.last_input_time {
            last_time.elapsed() >= Duration::from_millis(self.debounce_ms)
        } else {
            false
        }
    }

    fn execute_search(&mut self) -> Result<()> {
        self.search_pending = false;

        let pattern = self.input.value();
        if pattern.trim().is_empty() {
            self.results.clear();
            self.searching = false;
            return Ok(());
        }

        // Build query filter
        let filter = QueryFilter {
            language: None, // TODO: Add language filter
            kind: None,     // TODO: Add kind filter
            use_ast: false,
            use_regex: self.filters.regex_mode,
            limit: Some(500),
            symbols_mode: self.filters.symbols_mode,
            expand: false,
            file_pattern: None,
            exact: false,
            use_contains: false,
            timeout_secs: 10,
            glob_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            paths_only: false,
            offset: None,
        };

        // Spawn background thread for search
        let (tx, rx) = mpsc::channel();
        let pattern_owned = pattern.to_string();
        let cache = CacheManager::new(&self.cwd);
        let engine = QueryEngine::new(cache);

        std::thread::spawn(move || {
            let result = engine.search_with_metadata(&pattern_owned, filter);
            tx.send(result).ok();
        });

        self.searching = true;
        self.search_rx = Some(rx);

        Ok(())
    }

    fn trigger_index(&mut self) -> Result<()> {
        self.index_status = IndexStatusState::Indexing {
            current: 0,
            total: 0,
            status: "Starting...".to_string(),
        };

        // Create channels for results and progress
        let (result_tx, result_rx) = mpsc::channel();
        let (progress_tx, progress_rx) = mpsc::channel();
        let cwd = self.cwd.clone();

        // Spawn background thread for indexing with progress callback
        std::thread::spawn(move || {
            let config = IndexConfig::default();
            let cache = CacheManager::new(&cwd);
            let indexer = Indexer::new(cache, config);

            // Create progress callback that sends updates through channel
            let callback = Arc::new(move |current: usize, total: usize, status: String| {
                let _ = progress_tx.send((current, total, status));
            });

            let result = indexer.index_with_callback(&cwd, false, Some(callback));
            result_tx.send(result).ok();
        });

        self.indexing = true;
        self.indexing_start_time = Some(Instant::now());
        self.index_rx = Some(result_rx);
        self.index_progress_rx = Some(progress_rx);

        Ok(())
    }

    fn open_in_editor_suspended(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        result: &SearchResult,
    ) -> Result<()> {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
        let line = result.span.start_line;

        // Build command with line number
        let args = match editor.as_str() {
            "vim" | "nvim" => vec![format!("+{}", line), result.path.clone()],
            "emacs" => vec![format!("+{}:0", line), result.path.clone()],
            "code" | "vscode" => vec!["-g".to_string(), format!("{}:{}", result.path, line)],
            _ => vec![result.path.clone()],
        };

        // Suspend terminal properly
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            terminal.backend_mut(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture,
            crossterm::cursor::Show
        )?;
        terminal.show_cursor()?;

        // Open editor
        let status = std::process::Command::new(&editor)
            .args(&args)
            .status()?;

        // Resume terminal
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(
            terminal.backend_mut(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;
        terminal.clear()?;

        if !status.success() {
            self.error_message = Some(format!("Editor exited with error code: {:?}", status.code()));
        }

        Ok(())
    }

    fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        crossterm::execute!(
            stdout,
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(terminal)
    }

    fn restore_terminal(mut terminal: Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            terminal.backend_mut(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        Ok(())
    }

    // Getters for UI rendering
    pub fn input(&self) -> &InputField {
        &self.input
    }

    pub fn results(&self) -> &ResultList {
        &self.results
    }

    pub fn mode(&self) -> &AppMode {
        &self.mode
    }

    pub fn filters(&self) -> &QueryFilters {
        &self.filters
    }

    pub fn capabilities(&self) -> &TerminalCapabilities {
        &self.capabilities
    }

    pub fn theme(&self) -> &ThemeManager {
        &self.theme
    }

    pub fn effects(&self) -> &EffectManager {
        &self.effects
    }

    pub fn index_status(&self) -> &IndexStatusState {
        &self.index_status
    }

    pub fn focus_state(&self) -> &FocusState {
        &self.focus_state
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    pub fn info_message(&self) -> Option<&str> {
        self.info_message.as_deref()
    }

    pub fn preview_content(&self) -> Option<&FilePreview> {
        self.preview_content.as_ref()
    }

    pub fn searching(&self) -> bool {
        self.searching
    }

    pub fn indexing(&self) -> bool {
        self.indexing
    }

    pub fn indexing_elapsed_secs(&self) -> Option<u64> {
        self.indexing_start_time.map(|start| start.elapsed().as_secs())
    }
}

impl FilePreview {
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn content(&self) -> &[String] {
        &self.content
    }

    pub fn visible_lines(&self, height: usize) -> &[String] {
        let start = self.scroll_offset;
        let end = (start + height).min(self.content.len());
        &self.content[start..end]
    }

    pub fn center_line(&self) -> usize {
        self.center_line
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }
}
