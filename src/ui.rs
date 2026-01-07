//! Terminal UI for meshgrid.
//!
//! Interactive terminal interface for monitoring and sending messages.

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use crate::device::MeshEvent;
use crate::protocol::{Protocol, MonitorEvent};
use crate::serial::SerialPort;

/// Message log entry.
#[derive(Debug, Clone)]
struct LogEntry {
    timestamp: String,
    content: String,
    style: Style,
}

/// Neighbor info for display.
#[derive(Debug, Clone)]
struct NeighborDisplay {
    name: String,
    rssi: i16,
    last_seen: std::time::Instant,
}

/// Application state.
struct App {
    /// Message log
    messages: Vec<LogEntry>,
    /// Input buffer
    input: String,
    /// Cursor position
    cursor: usize,
    /// Neighbors map (node_hash -> display info)
    neighbors: HashMap<u8, NeighborDisplay>,
    /// Device name
    device_name: String,
    /// Should quit
    should_quit: bool,
}

impl App {
    fn new(device_name: String) -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            cursor: 0,
            neighbors: HashMap::new(),
            device_name,
            should_quit: false,
        }
    }

    fn add_message(&mut self, content: String, style: Style) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        self.messages.push(LogEntry {
            timestamp,
            content,
            style,
        });

        // Keep max 1000 messages
        if self.messages.len() > 1000 {
            self.messages.remove(0);
        }
    }

    fn add_info(&mut self, content: String) {
        self.add_message(content, Style::default().fg(Color::Cyan));
    }

    fn add_received(&mut self, from: &str, text: &str, rssi: i16) {
        let content = format!("{} ({}dB): {}", from, rssi, text);
        self.add_message(content, Style::default().fg(Color::Green));
    }

    fn add_sent(&mut self, text: &str) {
        let content = format!("You: {}", text);
        self.add_message(content, Style::default().fg(Color::Yellow));
    }

    fn add_error(&mut self, content: String) {
        self.add_message(content, Style::default().fg(Color::Red));
    }

    fn update_neighbor(&mut self, node_hash: u8, name: Option<String>, rssi: i16) {
        let display_name = name.unwrap_or_else(|| format!("0x{:02x}", node_hash));
        self.neighbors.insert(node_hash, NeighborDisplay {
            name: display_name,
            rssi,
            last_seen: std::time::Instant::now(),
        });

        // Remove stale neighbors (not seen in 5 minutes)
        let cutoff = std::time::Instant::now() - std::time::Duration::from_secs(300);
        self.neighbors.retain(|_, v| v.last_seen > cutoff);
    }
}

/// Run the terminal UI.
pub async fn run(port: &str, baud: u32) -> Result<()> {
    // Connect to device - get info first
    let serial = SerialPort::open(port, baud).await?;
    let mut protocol = Protocol::new(serial);

    // Get device info
    let info = protocol.get_info().await?;
    let device_name = info.name.clone().unwrap_or_else(|| format!("0x{:02x}", info.node_hash));

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let app = Arc::new(Mutex::new(App::new(device_name)));
    app.lock().unwrap().add_info(format!(
        "Connected to {} on {}",
        info.name.as_deref().unwrap_or("device"),
        port
    ));
    app.lock().unwrap().add_info("Type a message and press Enter to send. Ctrl+Q to quit.".into());

    // Create channels for communication
    let (tx_event, mut rx_event) = mpsc::channel::<MeshEvent>(100);
    let (tx_cmd, mut rx_cmd) = mpsc::channel::<String>(10);

    // Spawn device handler task
    let app_clone = app.clone();
    let device_task = tokio::spawn(async move {
        // Enter monitor mode and handle events
        if let Err(e) = protocol.enter_monitor_mode().await {
            app_clone.lock().unwrap().add_error(format!("Monitor error: {}", e));
            return;
        }

        loop {
            tokio::select! {
                // Check for mesh events
                result = protocol.read_event() => {
                    match result {
                        Ok(Some(event)) => {
                            let _ = tx_event.send(match event {
                                MonitorEvent::Message { from, to, rssi, snr, text } => {
                                    MeshEvent::Message { from, to, text, rssi, snr }
                                }
                                MonitorEvent::Advertisement { node_hash, rssi, name } => {
                                    MeshEvent::Advertisement { node_hash, rssi, name }
                                }
                                MonitorEvent::Ack { from } => {
                                    MeshEvent::Ack { from }
                                }
                                MonitorEvent::Error { message } => {
                                    MeshEvent::Error { message }
                                }
                            }).await;
                        }
                        Ok(None) => {}
                        Err(e) => {
                            app_clone.lock().unwrap().add_error(format!("Read error: {}", e));
                            break;
                        }
                    }
                }
                // Check for commands to send
                cmd = rx_cmd.recv() => {
                    match cmd {
                        Some(msg) => {
                            if let Err(e) = protocol.send_broadcast(&msg).await {
                                app_clone.lock().unwrap().add_error(format!("Send error: {}", e));
                            }
                        }
                        None => break,
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    });

    // Main UI loop
    let result = run_ui_loop(&mut terminal, app.clone(), &mut rx_event, &tx_cmd).await;

    // Clean up
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Wait for device task
    device_task.abort();

    result
}

async fn run_ui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: Arc<Mutex<App>>,
    rx_event: &mut mpsc::Receiver<MeshEvent>,
    tx_cmd: &mpsc::Sender<String>,
) -> Result<()> {
    loop {
        // Draw UI
        {
            let app = app.lock().unwrap();
            terminal.draw(|f| draw_ui(f, &app))?;
        }

        // Check for mesh events (non-blocking)
        while let Ok(event) = rx_event.try_recv() {
            let mut app = app.lock().unwrap();
            match event {
                MeshEvent::Message { from, to, text, rssi, snr: _ } => {
                    let dest = to.as_deref().unwrap_or("all");
                    app.add_received(&from, &format!("[->{}] {}", dest, text), rssi);
                }
                MeshEvent::Advertisement { node_hash, rssi, name } => {
                    app.update_neighbor(node_hash, name.clone(), rssi);
                    let display_name = name.unwrap_or_else(|| format!("0x{:02x}", node_hash));
                    app.add_info(format!("ADV: {} ({}dB)", display_name, rssi));
                }
                MeshEvent::Ack { from } => {
                    app.add_info(format!("ACK from {}", from));
                }
                MeshEvent::Error { message } => {
                    app.add_error(message);
                }
            }
        }

        // Check for quit
        if app.lock().unwrap().should_quit {
            return Ok(());
        }

        // Handle keyboard input (with timeout)
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                let mut app = app.lock().unwrap();

                match key.code {
                    KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.should_quit = true;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.should_quit = true;
                    }
                    KeyCode::Enter => {
                        if !app.input.is_empty() {
                            let msg = app.input.clone();
                            app.add_sent(&msg);
                            app.input.clear();
                            app.cursor = 0;

                            // Send in background
                            let _ = tx_cmd.send(msg).await;
                        }
                    }
                    KeyCode::Char(c) => {
                        let cursor = app.cursor;
                        app.input.insert(cursor, c);
                        app.cursor += 1;
                    }
                    KeyCode::Backspace => {
                        if app.cursor > 0 {
                            app.cursor -= 1;
                            let cursor = app.cursor;
                            app.input.remove(cursor);
                        }
                    }
                    KeyCode::Delete => {
                        let cursor = app.cursor;
                        if cursor < app.input.len() {
                            app.input.remove(cursor);
                        }
                    }
                    KeyCode::Left => {
                        if app.cursor > 0 {
                            app.cursor -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if app.cursor < app.input.len() {
                            app.cursor += 1;
                        }
                    }
                    KeyCode::Home => {
                        app.cursor = 0;
                    }
                    KeyCode::End => {
                        app.cursor = app.input.len();
                    }
                    _ => {}
                }
            }
        }
    }
}

fn draw_ui(f: &mut Frame, app: &App) {
    // Create main layout: header, content, input
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Content (messages + neighbors)
            Constraint::Length(3), // Input
        ])
        .split(f.size());

    // Header
    let neighbor_count = app.neighbors.len();
    let header_text = format!(" meshgrid - {} | {} neighbors ", app.device_name, neighbor_count);
    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, main_chunks[0]);

    // Split content area: messages (left) + neighbors (right)
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(75), // Messages
            Constraint::Percentage(25), // Neighbors
        ])
        .split(main_chunks[1]);

    // Messages panel
    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .rev()
        .take(content_chunks[0].height as usize - 2)
        .rev()
        .map(|entry| {
            let content = Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(&entry.content, entry.style),
            ]);
            ListItem::new(content)
        })
        .collect();

    let messages_list = List::new(messages)
        .block(Block::default().title(" Messages ").borders(Borders::ALL));
    f.render_widget(messages_list, content_chunks[0]);

    // Neighbors panel
    let mut neighbors: Vec<_> = app.neighbors.iter().collect();
    neighbors.sort_by(|a, b| b.1.rssi.cmp(&a.1.rssi)); // Sort by signal strength

    let neighbor_items: Vec<ListItem> = neighbors
        .iter()
        .take(content_chunks[1].height as usize - 2)
        .map(|(_, info)| {
            let age_secs = info.last_seen.elapsed().as_secs();
            let age_str = if age_secs < 60 {
                format!("{}s", age_secs)
            } else {
                format!("{}m", age_secs / 60)
            };

            let rssi_color = if info.rssi > -70 {
                Color::Green
            } else if info.rssi > -90 {
                Color::Yellow
            } else {
                Color::Red
            };

            let content = Line::from(vec![
                Span::styled(
                    format!("{:>4}dB ", info.rssi),
                    Style::default().fg(rssi_color),
                ),
                Span::raw(&info.name),
                Span::styled(
                    format!(" ({})", age_str),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            ListItem::new(content)
        })
        .collect();

    let neighbors_list = List::new(neighbor_items)
        .block(Block::default().title(" Neighbors ").borders(Borders::ALL));
    f.render_widget(neighbors_list, content_chunks[1]);

    // Input
    let input = Paragraph::new(app.input.as_str())
        .style(Style::default())
        .block(Block::default().title(" Send (Enter) | Ctrl+Q quit ").borders(Borders::ALL));
    f.render_widget(input, main_chunks[2]);

    // Set cursor position
    f.set_cursor(
        main_chunks[2].x + app.cursor as u16 + 1,
        main_chunks[2].y + 1,
    );
}
