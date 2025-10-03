use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use thiserror::Error;
use tokio::sync::mpsc;

use crate::cache::CacheStats;
use crate::streaming::StreamToken;

use pulldown_cmark::{Parser, Event as MarkdownEvent, Tag, CodeBlockKind};

#[derive(Debug, Error)]
pub enum UIError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Terminal error: {0}")]
    Terminal(String),
    #[error("Event handling error: {0}")]
    Event(String),
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub model: String,
    pub template_used: Option<String>,
    pub cached: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl MessageRole {
    pub fn as_str(&self) -> &str {
        match self {
            MessageRole::User => "User",
            MessageRole::Assistant => "Assistant",
            MessageRole::System => "System",
        }
    }

    pub fn color(&self, high_contrast: bool) -> Color {
        if high_contrast {
            match self {
                MessageRole::User => Color::White,
                MessageRole::Assistant => Color::White,
                MessageRole::System => Color::White,
            }
        } else {
            match self {
                MessageRole::User => Color::Cyan,
                MessageRole::Assistant => Color::Green,
                MessageRole::System => Color::Yellow,
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub current_model: String,
    pub is_streaming: bool,
    pub cache_stats: CacheStats,
    pub active_template: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_model: "llama3.2".to_string(),
            is_streaming: false,
            cache_stats: CacheStats {
                hits: 0,
                misses: 0,
                total_entries: 0,
                memory_usage_bytes: 0,
                evictions: 0,
                disk_writes: 0,
                disk_reads: 0,
            },
            active_template: None,
        }
    }
}

pub enum UIAction {
    SendMessage(String),
    ChangeModel(String),
    LoadTemplate(String),
    ClearHistory,
    Quit,
    None,
}

#[derive(Debug, Clone)]
struct ThemeColors {
    primary: Color,
    secondary: Color,
    accent: Color,
    background: Color,
    text: Color,
    border: Color,
    success: Color,
    warning: Color,
    error: Color,
    info: Color,
}

pub struct TerminalUI {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    app_state: AppState,
    message_history: Vec<ChatMessage>,
    input_buffer: String,
    scroll_offset: usize,
    current_streaming_content: String,
    markdown_renderer: MarkdownRenderer,
    auto_scroll: bool,
    progress_animation_frame: usize,
    high_contrast_mode: bool,
    last_terminal_size: (u16, u16),
}

pub struct MarkdownRenderer {
    // Simple syntax highlighting without external dependencies
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render_to_spans(&self, content: &str) -> Vec<Span> {
        let mut spans = Vec::new();
        let parser = Parser::new(content);
        let mut in_code_block = false;
        let mut code_language = String::new();
        let mut code_content = String::new();

        for event in parser {
            match event {
                MarkdownEvent::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                    in_code_block = true;
                    code_language = lang.to_string();
                    code_content.clear();
                }
                MarkdownEvent::End(Tag::CodeBlock(_)) => {
                    if in_code_block {
                        spans.extend(self.highlight_code(&code_content, &code_language));
                        in_code_block = false;
                    }
                }
                MarkdownEvent::Text(text) => {
                    if in_code_block {
                        code_content.push_str(&text);
                    } else {
                        spans.push(Span::raw(text.to_string()));
                    }
                }
                MarkdownEvent::Code(code) => {
                    spans.push(Span::styled(
                        code.to_string(),
                        Style::default().fg(Color::Yellow).bg(Color::DarkGray)
                    ));
                }
                MarkdownEvent::Start(Tag::Strong) => {
                    // We'll handle this in a more sophisticated way later
                }
                MarkdownEvent::End(Tag::Strong) => {
                    // We'll handle this in a more sophisticated way later
                }
                MarkdownEvent::Start(Tag::Emphasis) => {
                    // We'll handle this in a more sophisticated way later
                }
                MarkdownEvent::End(Tag::Emphasis) => {
                    // We'll handle this in a more sophisticated way later
                }
                MarkdownEvent::SoftBreak | MarkdownEvent::HardBreak => {
                    spans.push(Span::raw("\n"));
                }
                _ => {}
            }
        }

        if spans.is_empty() {
            spans.push(Span::raw(content.to_string()));
        }

        spans
    }

    fn highlight_code(&self, code: &str, language: &str) -> Vec<Span> {
        let mut spans = Vec::new();
        
        // Add code block header
        spans.push(Span::styled(
            format!("```{}\n", language),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        ));

        // Simple syntax highlighting based on language
        let code_color = match language.to_lowercase().as_str() {
            "rust" | "rs" => Color::Red,
            "python" | "py" => Color::Blue,
            "javascript" | "js" | "typescript" | "ts" => Color::Yellow,
            "json" => Color::Green,
            "html" | "xml" => Color::Magenta,
            "css" => Color::Cyan,
            "bash" | "sh" | "shell" => Color::Gray,
            _ => Color::White,
        };

        // Apply basic highlighting
        for line in code.lines() {
            // Simple keyword highlighting for common languages
            if language.to_lowercase().as_str() == "rust" {
                spans.extend(self.highlight_rust_line(line));
            } else if language.to_lowercase().as_str() == "python" {
                spans.extend(self.highlight_python_line(line));
            } else {
                // Default: just color the whole line
                spans.push(Span::styled(
                    line.to_string(),
                    Style::default().fg(code_color).bg(Color::DarkGray)
                ));
            }
            spans.push(Span::raw("\n"));
        }

        // Add code block footer
        spans.push(Span::styled(
            "```\n",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        ));

        spans
    }

    fn highlight_rust_line(&self, line: &str) -> Vec<Span> {
        let mut spans = Vec::new();
        let keywords = ["fn", "let", "mut", "pub", "struct", "impl", "use", "mod", "if", "else", "match", "for", "while", "loop"];
        
        let mut current_word = String::new();
        let mut in_string = false;
        let mut chars = line.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '"' && !in_string {
                in_string = true;
                if !current_word.is_empty() {
                    spans.push(self.style_word(&current_word, &keywords));
                    current_word.clear();
                }
                current_word.push(ch);
            } else if ch == '"' && in_string {
                in_string = false;
                current_word.push(ch);
                spans.push(Span::styled(current_word.clone(), Style::default().fg(Color::Green).bg(Color::DarkGray)));
                current_word.clear();
            } else if in_string {
                current_word.push(ch);
            } else if ch.is_whitespace() || "(){}[];,".contains(ch) {
                if !current_word.is_empty() {
                    spans.push(self.style_word(&current_word, &keywords));
                    current_word.clear();
                }
                spans.push(Span::styled(ch.to_string(), Style::default().fg(Color::White).bg(Color::DarkGray)));
            } else {
                current_word.push(ch);
            }
        }
        
        if !current_word.is_empty() {
            spans.push(self.style_word(&current_word, &keywords));
        }
        
        spans
    }

    fn highlight_python_line(&self, line: &str) -> Vec<Span> {
        let mut spans = Vec::new();
        let keywords = ["def", "class", "if", "else", "elif", "for", "while", "try", "except", "import", "from", "return", "yield"];
        
        let mut current_word = String::new();
        let mut in_string = false;
        let mut chars = line.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if (ch == '"' || ch == '\'') && !in_string {
                in_string = true;
                if !current_word.is_empty() {
                    spans.push(self.style_word(&current_word, &keywords));
                    current_word.clear();
                }
                current_word.push(ch);
            } else if (ch == '"' || ch == '\'') && in_string {
                in_string = false;
                current_word.push(ch);
                spans.push(Span::styled(current_word.clone(), Style::default().fg(Color::Green).bg(Color::DarkGray)));
                current_word.clear();
            } else if in_string {
                current_word.push(ch);
            } else if ch.is_whitespace() || "(){}[];,:".contains(ch) {
                if !current_word.is_empty() {
                    spans.push(self.style_word(&current_word, &keywords));
                    current_word.clear();
                }
                spans.push(Span::styled(ch.to_string(), Style::default().fg(Color::White).bg(Color::DarkGray)));
            } else {
                current_word.push(ch);
            }
        }
        
        if !current_word.is_empty() {
            spans.push(self.style_word(&current_word, &keywords));
        }
        
        spans
    }

    fn style_word(&self, word: &str, keywords: &[&str]) -> Span {
        if keywords.contains(&word) {
            Span::styled(word.to_string(), Style::default().fg(Color::Magenta).bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        } else if word.chars().all(|c| c.is_ascii_digit()) {
            Span::styled(word.to_string(), Style::default().fg(Color::Yellow).bg(Color::DarkGray))
        } else {
            Span::styled(word.to_string(), Style::default().fg(Color::White).bg(Color::DarkGray))
        }
    }
}

impl TerminalUI {
    pub fn new() -> Result<Self, UIError> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let initial_size = terminal.size()?;
        
        Ok(Self {
            terminal,
            app_state: AppState::default(),
            message_history: Vec::new(),
            input_buffer: String::new(),
            scroll_offset: 0,
            current_streaming_content: String::new(),
            markdown_renderer: MarkdownRenderer::new(),
            auto_scroll: true,
            progress_animation_frame: 0,
            high_contrast_mode: false,
            last_terminal_size: (initial_size.width, initial_size.height),
        })
    }

    pub async fn run(&mut self, mut stream_receiver: mpsc::UnboundedReceiver<StreamToken>) -> Result<(), UIError> {
        loop {
            self.render_frame()?;

            // Handle events with timeout
            if event::poll(std::time::Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => {
                        match self.handle_input(key) {
                            UIAction::Quit => break,
                            UIAction::SendMessage(msg) => {
                                self.add_message(ChatMessage {
                                    role: MessageRole::User,
                                    content: msg,
                                    timestamp: chrono::Utc::now(),
                                    model: self.app_state.current_model.clone(),
                                    template_used: self.app_state.active_template.clone(),
                                    cached: false,
                                });
                                self.input_buffer.clear();
                            }
                            UIAction::ClearHistory => {
                                self.message_history.clear();
                                self.scroll_offset = 0;
                            }
                            UIAction::ChangeModel(model) => {
                                self.app_state.current_model = model;
                            }
                            _ => {}
                        }
                    }
                    Event::Resize(width, height) => {
                        // Terminal was resized, update our tracking and adapt layout
                        self.last_terminal_size = (width, height);
                        
                        // Force a re-render to adapt to new size
                        self.terminal.resize(ratatui::layout::Rect {
                            x: 0,
                            y: 0,
                            width,
                            height,
                        })?;
                        
                        // Adjust scroll offset if needed to prevent going out of bounds
                        let max_scroll = self.message_history.len().saturating_sub(1);
                        if self.scroll_offset > max_scroll {
                            self.scroll_offset = max_scroll;
                        }
                    }
                    _ => {}
                }
            }

            // Handle streaming tokens
            while let Ok(token) = stream_receiver.try_recv() {
                self.update_streaming_content(token);
            }

            // Small delay to prevent excessive CPU usage
            tokio::time::sleep(tokio::time::Duration::from_millis(16)).await; // ~60 FPS
        }

        self.cleanup()?;
        Ok(())
    }

    pub fn handle_input(&mut self, key: KeyEvent) -> UIAction {
        match key.code {
            // Quit commands
            KeyCode::Char('q') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                UIAction::Quit
            }
            KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                UIAction::Quit
            }
            KeyCode::Esc => {
                UIAction::Quit
            }
            
            // Clear history
            KeyCode::Char('l') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                UIAction::ClearHistory
            }
            
            // Send message
            KeyCode::Enter => {
                if !self.input_buffer.trim().is_empty() {
                    UIAction::SendMessage(self.input_buffer.clone())
                } else {
                    UIAction::None
                }
            }
            
            // Input editing
            KeyCode::Backspace => {
                self.input_buffer.pop();
                UIAction::None
            }
            KeyCode::Delete => {
                // For now, just treat as backspace
                self.input_buffer.pop();
                UIAction::None
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
                UIAction::None
            }
            
            // Navigation
            KeyCode::Up => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
                UIAction::None
            }
            KeyCode::Down => {
                if self.scroll_offset < self.message_history.len().saturating_sub(1) {
                    self.scroll_offset += 1;
                }
                UIAction::None
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
                UIAction::None
            }
            KeyCode::PageDown => {
                let max_scroll = self.message_history.len().saturating_sub(1);
                self.scroll_offset = (self.scroll_offset + 10).min(max_scroll);
                UIAction::None
            }
            KeyCode::Home => {
                self.scroll_offset = 0;
                UIAction::None
            }
            KeyCode::End => {
                self.scroll_offset = self.message_history.len().saturating_sub(1);
                UIAction::None
            }
            
            // Model switching (F1-F4 for quick model selection)
            KeyCode::F(1) => UIAction::ChangeModel("llama3.2".to_string()),
            KeyCode::F(2) => UIAction::ChangeModel("codellama".to_string()),
            KeyCode::F(3) => UIAction::ChangeModel("mistral".to_string()),
            KeyCode::F(4) => UIAction::ChangeModel("phi3".to_string()),
            
            // Toggle auto-scroll
            KeyCode::F(5) => {
                self.auto_scroll = !self.auto_scroll;
                UIAction::None
            }
            
            // Toggle high contrast mode
            KeyCode::F(6) => {
                self.high_contrast_mode = !self.high_contrast_mode;
                UIAction::None
            }
            
            _ => UIAction::None,
        }
    }

    pub fn render_frame(&mut self) -> Result<(), UIError> {
        let app_state = self.app_state.clone();
        let message_history = self.message_history.clone();
        let input_buffer = self.input_buffer.clone();
        let current_streaming_content = self.current_streaming_content.clone();
        let progress_indicator = self.get_progress_indicator();
        let high_contrast = self.high_contrast_mode;
        
        // Update animation frame for smooth progress indicator
        if self.app_state.is_streaming {
            self.progress_animation_frame = (self.progress_animation_frame + 1) % 4;
        }
        
        self.terminal.draw(|f| {
            // Handle responsive layout based on terminal size
            let size = f.size();
            let constraints = if size.height < 10 {
                // Minimal layout for very small terminals
                vec![
                    Constraint::Length(1),  // Minimal status
                    Constraint::Min(0),     // Chat history
                    Constraint::Length(1),  // Minimal input
                ]
            } else if size.height < 20 {
                // Compact layout for small terminals
                vec![
                    Constraint::Length(2),  // Compact status
                    Constraint::Min(0),     // Chat history
                    Constraint::Length(2),  // Compact input
                ]
            } else {
                // Full layout for normal terminals
                vec![
                    Constraint::Length(3),  // Status bar
                    Constraint::Min(0),     // Chat history
                    Constraint::Length(3),  // Input area
                ]
            };

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(size);

            Self::render_status_bar_static(f, chunks[0], &app_state, progress_indicator, high_contrast);
            Self::render_chat_history_with_renderer(f, chunks[1], &message_history, &current_streaming_content, &self.markdown_renderer, high_contrast, progress_indicator);
            Self::render_input_area_static(f, chunks[2], &input_buffer, high_contrast);
        })?;

        Ok(())
    }

    fn render_status_bar_static(f: &mut Frame, area: Rect, app_state: &AppState, progress_indicator: &str, high_contrast: bool) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
            .split(area);

        let theme = if high_contrast {
            ThemeColors {
                primary: Color::White,
                secondary: Color::Black,
                accent: Color::White,
                background: Color::Black,
                text: Color::White,
                border: Color::White,
                success: Color::White,
                warning: Color::White,
                error: Color::White,
                info: Color::White,
            }
        } else {
            ThemeColors {
                primary: Color::Cyan,
                secondary: Color::Blue,
                accent: Color::Yellow,
                background: Color::Black,
                text: Color::White,
                border: Color::Gray,
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Blue,
            }
        };

        // Main status info
        let status_text = format!(
            "Model: {} | Streaming: {} {} | Cache: {:.1}% hit rate ({} entries) | Template: {} | Mode: {}",
            app_state.current_model,
            progress_indicator,
            if app_state.is_streaming { "Live" } else { "Ready" },
            app_state.cache_stats.hit_ratio() * 100.0,
            app_state.cache_stats.total_entries,
            app_state.active_template.as_deref().unwrap_or("None"),
            if high_contrast { "High Contrast" } else { "Normal" }
        );

        let status = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .style(Style::default().fg(theme.text))
            .wrap(Wrap { trim: true });

        f.render_widget(status, chunks[0]);

        // Keyboard shortcuts
        let shortcuts = Paragraph::new("Ctrl+Q: Quit | Ctrl+L: Clear | F1-F4: Models | F5: Auto-scroll | F6: High contrast | ↑↓: Scroll")
            .block(Block::default().borders(Borders::ALL).title("Shortcuts"))
            .style(Style::default().fg(if high_contrast { Color::White } else { Color::Gray }))
            .wrap(Wrap { trim: true });

        f.render_widget(shortcuts, chunks[1]);
    }

    fn render_chat_history_static(f: &mut Frame, area: Rect, message_history: &[ChatMessage], current_streaming_content: &str) {
        Self::render_chat_history_with_renderer(f, area, message_history, current_streaming_content, &MarkdownRenderer::new(), false, "⚫");
    }

    fn render_chat_history_with_renderer(
        f: &mut Frame, 
        area: Rect, 
        message_history: &[ChatMessage], 
        current_streaming_content: &str,
        renderer: &MarkdownRenderer,
        high_contrast: bool,
        progress_indicator: &str
    ) {
        let messages: Vec<ListItem> = message_history
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                let timestamp = msg.timestamp.format("%H:%M:%S");
                let mut spans = vec![
                    Span::styled(
                        format!("[{}] {}: ", timestamp, msg.role.as_str()), 
                        Style::default().fg(msg.role.color(high_contrast)).add_modifier(Modifier::BOLD)
                    )
                ];
                
                // Add indicators
                if msg.cached {
                    spans.push(Span::styled("📋 ", Style::default().fg(if high_contrast { Color::White } else { Color::Blue })));
                }
                if msg.template_used.is_some() {
                    spans.push(Span::styled("📝 ", Style::default().fg(if high_contrast { Color::White } else { Color::Magenta })));
                }
                
                // Render message content with markdown support
                if msg.role == MessageRole::Assistant && (msg.content.contains("```") || msg.content.contains("`")) {
                    // Use markdown rendering for assistant messages that might contain code
                    let mut content_spans = renderer.render_to_spans(&msg.content);
                    spans.append(&mut content_spans);
                } else {
                    // For user messages or simple text, just add as raw text but handle line breaks
                    let content = if msg.content.len() > 200 {
                        format!("{}...", &msg.content[..197])
                    } else {
                        msg.content.clone()
                    };
                    
                    // Handle line breaks in content
                    for (line_idx, line) in content.lines().enumerate() {
                        if line_idx > 0 {
                            spans.push(Span::raw("\n"));
                        }
                        spans.push(Span::raw(line.to_string()));
                    }
                }
                
                // Add message number for reference
                spans.push(Span::styled(
                    format!(" #{}", i + 1),
                    Style::default().fg(Color::DarkGray)
                ));
                
                ListItem::new(Line::from(spans))
            })
            .collect();

        // Add current streaming content if any
        let mut all_messages = messages;
        if !current_streaming_content.is_empty() {
            let timestamp = chrono::Utc::now().format("%H:%M:%S");
            let mut streaming_spans = vec![
                Span::styled(format!("[{}] Assistant: ", timestamp), Style::default().fg(if high_contrast { Color::White } else { Color::Green }).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{} ", progress_indicator), Style::default().fg(if high_contrast { Color::White } else { Color::Yellow })),
            ];
            
            // Apply markdown rendering to streaming content if it contains code
            if current_streaming_content.contains("```") || current_streaming_content.contains("`") {
                let mut content_spans = renderer.render_to_spans(current_streaming_content);
                streaming_spans.append(&mut content_spans);
            } else {
                streaming_spans.push(Span::raw(current_streaming_content.to_string()));
            }
            
            let streaming_item = ListItem::new(Line::from(streaming_spans));
            all_messages.push(streaming_item);
        }

        let title = if message_history.is_empty() {
            "Chat History (No messages yet - start typing below!)"
        } else {
            &format!("Chat History ({} messages) - Markdown & syntax highlighting enabled", message_history.len())
        };

        let messages_list = List::new(all_messages)
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(Style::default().fg(if high_contrast { Color::White } else { Color::White }));

        f.render_widget(messages_list, area);
    }

    fn render_input_area_static(f: &mut Frame, area: Rect, input_buffer: &str, high_contrast: bool) {
        let title = if input_buffer.is_empty() {
            "Input (Type your message and press Enter to send)"
        } else {
            &format!("Input ({} chars) - Press Enter to send", input_buffer.len())
        };

        let text_color = if high_contrast { Color::White } else { Color::White };
        let border_color = if high_contrast { Color::White } else { Color::Gray };

        let input = Paragraph::new(input_buffer)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border_color)))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(text_color));

        f.render_widget(input, area);

        // Set cursor position - handle wrapping for long input
        let cursor_x = if input_buffer.len() as u16 + 1 < area.width - 2 {
            area.x + input_buffer.len() as u16 + 1
        } else {
            area.x + (area.width - 2)
        };

        f.set_cursor(cursor_x, area.y + 1);
    }

    pub fn update_streaming_content(&mut self, token: StreamToken) {
        if token.is_complete {
            // Streaming is complete, add the final message
            self.add_message(ChatMessage {
                role: MessageRole::Assistant,
                content: self.current_streaming_content.clone(),
                timestamp: chrono::Utc::now(),
                model: self.app_state.current_model.clone(),
                template_used: self.app_state.active_template.clone(),
                cached: false,
            });
            self.current_streaming_content.clear();
            self.app_state.is_streaming = false;
        } else {
            // Accumulate streaming content
            self.current_streaming_content.push_str(&token.content);
            self.app_state.is_streaming = true;
            // Update animation frame for progress indicator
            self.progress_animation_frame = (self.progress_animation_frame + 1) % 4;
        }
    }

    fn get_progress_indicator(&self) -> &'static str {
        if self.app_state.is_streaming {
            match self.progress_animation_frame {
                0 => "⠋",
                1 => "⠙",
                2 => "⠹",
                3 => "⠸",
                _ => "⠋",
            }
        } else {
            "⚫"
        }
    }

    fn get_theme_colors(&self) -> ThemeColors {
        if self.high_contrast_mode {
            ThemeColors {
                primary: Color::White,
                secondary: Color::Black,
                accent: Color::White,
                background: Color::Black,
                text: Color::White,
                border: Color::White,
                success: Color::White,
                warning: Color::White,
                error: Color::White,
                info: Color::White,
            }
        } else {
            ThemeColors {
                primary: Color::Cyan,
                secondary: Color::Blue,
                accent: Color::Yellow,
                background: Color::Black,
                text: Color::White,
                border: Color::Gray,
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Blue,
            }
        }
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        self.message_history.push(message);
        // Auto-scroll to bottom if enabled
        if self.auto_scroll {
            self.scroll_offset = self.message_history.len().saturating_sub(1);
        }
    }

    pub fn update_app_state(&mut self, state: AppState) {
        self.app_state = state;
    }

    fn cleanup(&mut self) -> Result<(), UIError> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for TerminalUI {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}