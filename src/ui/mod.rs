use crate::navigation::GnssPosition;
use crate::receiver::GnssReceiver;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table},
    Frame, Terminal,
};
use std::io;
use tokio::time::{Duration, interval};

pub struct UI {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

#[derive(Default)]
pub struct UiState {
    pub position: Option<GnssPosition>,
    pub satellites: Vec<SatelliteView>,
    pub status_message: String,
}

#[derive(Clone)]
pub struct SatelliteView {
    pub prn: u8,
    pub elevation: f64,
    pub azimuth: f64,
    pub cn0: f64,
    pub state: String,
    pub used_in_fix: bool,
}

impl UI {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self { terminal })
    }

    pub async fn run(&mut self, receiver: &mut GnssReceiver) -> io::Result<()> {
        let mut ui_state = UiState::default();
        let mut tick_interval = interval(Duration::from_millis(100));

        loop {
            // Update UI state from receiver
            self.update_state(receiver, &mut ui_state);

            // Draw UI
            self.terminal.draw(|f| Self::render(f, &ui_state))?;

            // Handle input
            if event::poll(Duration::from_millis(10))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            break;
                        }
                        _ => {}
                    }
                }
            }

            tick_interval.tick().await;
        }

        Ok(())
    }

    fn update_state(&self, receiver: &GnssReceiver, state: &mut UiState) {
        // Update state from receiver
        state.position = receiver.get_position();
        state.satellites = receiver.get_satellite_views();
        state.status_message = receiver.get_status();
    }

    fn render(f: &mut Frame, state: &UiState) {
        let size = f.size();

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // Position info
                Constraint::Min(10),    // Satellite table
                Constraint::Length(3),  // Status bar
            ])
            .split(size);

        // Render position info
        Self::render_position(f, chunks[0], state);

        // Render satellite table
        Self::render_satellites(f, chunks[1], state);

        // Render status bar
        Self::render_status(f, chunks[2], state);
    }

    fn render_position(f: &mut Frame, area: Rect, state: &UiState) {
        let text = if let Some(ref pos) = state.position {
            vec![
                Line::from(vec![
                    Span::styled("Latitude:  ", Style::default().fg(Color::Yellow)),
                    Span::raw(format!("{:.7}°", pos.latitude)),
                ]),
                Line::from(vec![
                    Span::styled("Longitude: ", Style::default().fg(Color::Yellow)),
                    Span::raw(format!("{:.7}°", pos.longitude)),
                ]),
                Line::from(vec![
                    Span::styled("Altitude:  ", Style::default().fg(Color::Yellow)),
                    Span::raw(format!("{:.2} m", pos.altitude)),
                ]),
                Line::from(vec![
                    Span::styled("Satellites:", Style::default().fg(Color::Yellow)),
                    Span::raw(format!(" {}", pos.num_satellites)),
                ]),
                Line::from(vec![
                    Span::styled("HDOP/VDOP: ", Style::default().fg(Color::Yellow)),
                    Span::raw(format!("{:.2} / {:.2}", pos.hdop, pos.vdop)),
                ]),
                Line::from(vec![
                    Span::styled("Clock Bias:", Style::default().fg(Color::Yellow)),
                    Span::raw(format!(" {:.3} m", pos.clock_bias)),
                ]),
            ]
        } else {
            vec![Line::from(Span::styled(
                "No position fix yet...",
                Style::default().fg(Color::Red),
            ))]
        };

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Position"));

        f.render_widget(paragraph, area);
    }

    fn render_satellites(f: &mut Frame, area: Rect, state: &UiState) {
        let header = Row::new(vec!["PRN", "Elevation", "Azimuth", "C/N0", "State", "Used"])
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .bottom_margin(1);

        let rows: Vec<Row> = state
            .satellites
            .iter()
            .map(|sat| {
                let style = if sat.used_in_fix {
                    Style::default().fg(Color::Green)
                } else if sat.state == "Tracking" {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::Gray)
                };

                Row::new(vec![
                    format!("{:3}", sat.prn),
                    format!("{:5.1}°", sat.elevation),
                    format!("{:6.1}°", sat.azimuth),
                    format!("{:5.1}", sat.cn0),
                    sat.state.clone(),
                    if sat.used_in_fix { "✓" } else { " " }.to_string(),
                ])
                .style(style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(4),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(6),
                Constraint::Length(10),
                Constraint::Length(5),
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Satellites"));

        f.render_widget(table, area);
    }

    fn render_status(f: &mut Frame, area: Rect, state: &UiState) {
        let status_text = vec![Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Yellow)),
            Span::raw(&state.status_message),
            Span::styled(" | Press 'q' to quit", Style::default().fg(Color::Gray)),
        ])];

        let paragraph = Paragraph::new(status_text).block(Block::default().borders(Borders::ALL));

        f.render_widget(paragraph, area);
    }
}

impl Drop for UI {
    fn drop(&mut self) {
        disable_raw_mode().ok();
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .ok();
        self.terminal.show_cursor().ok();
    }
}
