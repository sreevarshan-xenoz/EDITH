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

#[derive(Debug, Clone)]
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

    pub fn color(&self) -> Color {
        match self {
            MessageRole::User => Color::Cyan,
            MessageRole::Assistant => Color::Green,
            MessageRole::System => Color::Yellow,
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

pub struct TerminalUI {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    app_state: AppState,
    message_history: Vec<ChatMessage>,
    input_buffer: String,
    scroll_offset: usize,
    current_streaming_content: String,
}

impl TerminalUI {
    pub fn new() -> Result<Self, UIError> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            terminal,
            app_state: AppState::default(),
            message_history: Vec::new(),
            input_buffer: String::new(),
            scroll_offset: 0,
            current_streaming_content: String::new(),
        })
    }

    pub async fn run(&mut self, mut stream_receiver: mpsc::UnboundedReceiver<StreamToken>) -> Result<(), UIError> {
        loop {
            self.render_frame()?;

            // Handle events with timeout
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
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
                        _ => {}
                    }
                }
            }

            // Handle streaming tokens
            while let Ok(token) = stream_receiver.try_recv() {
                self.update_streaming_content(token);
            }
        }

        self.cleanup()?;
        Ok(())
    }

    pub fn handle_input(&mut self, key: KeyEvent) -> UIAction {
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                UIAction::Quit
            }
            KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                UIAction::Quit
            }
            KeyCode::Char('l') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                UIAction::ClearHistory
            }
            KeyCode::Enter => {
                if !self.input_buffer.trim().is_empty() {
                    UIAction::SendMessage(self.input_buffer.clone())
                } else {
                    UIAction::None
                }
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
                UIAction::None
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
                UIAction::None
            }
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
            _ => UIAction::None,
        }
    }

    pub fn render_frame(&mut self) -> Result<(), UIError> {
        let app_state = self.app_state.clone();
        let message_history = self.message_history.clone();
        let input_buffer = self.input_buffer.clone();
        let current_streaming_content = self.current_streaming_content.clone();
        
        self.terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // Status bar
                    Constraint::Min(0),     // Chat history
                    Constraint::Length(3),  // Input area
                ].as_ref())
                .split(f.size());

            Self::render_status_bar_static(f, chunks[0], &app_state);
            Self::render_chat_history_static(f, chunks[1], &message_history, &current_streaming_content);
            Self::render_input_area_static(f, chunks[2], &input_buffer);
        })?;

        Ok(())
    }

    fn render_status_bar_static(f: &mut Frame, area: Rect, app_state: &AppState) {
        let status_text = format!(
            "Model: {} | Streaming: {} | Cache: {:.1}% hit rate | Template: {}",
            app_state.current_model,
            if app_state.is_streaming { "🔴" } else { "⚫" },
            app_state.cache_stats.hit_ratio() * 100.0,
            app_state.active_template.as_deref().unwrap_or("None")
        );

        let status = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .style(Style::default().fg(Color::White));

        f.render_widget(status, area);
    }

    fn render_chat_history_static(f: &mut Frame, area: Rect, message_history: &[ChatMessage], current_streaming_content: &str) {
        let messages: Vec<ListItem> = message_history
            .iter()
            .enumerate()
            .map(|(_i, msg)| {
                let timestamp = msg.timestamp.format("%H:%M:%S");
                let prefix = format!("[{}] {}: ", timestamp, msg.role.as_str());
                
                let mut spans = vec![
                    Span::styled(prefix, Style::default().fg(msg.role.color()).add_modifier(Modifier::BOLD))
                ];
                
                // Add cached indicator
                if msg.cached {
                    spans.push(Span::styled("📋 ", Style::default().fg(Color::Blue)));
                }
                
                spans.push(Span::raw(&msg.content));
                
                ListItem::new(Line::from(spans))
            })
            .collect();

        // Add current streaming content if any
        let mut all_messages = messages;
        if !current_streaming_content.is_empty() {
            let streaming_item = ListItem::new(Line::from(vec![
                Span::styled("Assistant: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled("⚡ ", Style::default().fg(Color::Yellow)),
                Span::raw(current_streaming_content),
            ]));
            all_messages.push(streaming_item);
        }

        let messages_list = List::new(all_messages)
            .block(Block::default().borders(Borders::ALL).title("Chat History"))
            .style(Style::default().fg(Color::White));

        f.render_widget(messages_list, area);
    }

    fn render_input_area_static(f: &mut Frame, area: Rect, input_buffer: &str) {
        let input = Paragraph::new(input_buffer)
            .block(Block::default().borders(Borders::ALL).title("Input (Enter to send, Ctrl+Q to quit)"))
            .wrap(Wrap { trim: true });

        f.render_widget(input, area);

        // Set cursor position
        f.set_cursor(
            area.x + input_buffer.len() as u16 + 1,
            area.y + 1,
        );
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
        }
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        self.message_history.push(message);
        // Auto-scroll to bottom
        self.scroll_offset = self.message_history.len().saturating_sub(1);
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