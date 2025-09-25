use std::sync::Arc;
use std::io;
use crate::error::ReplayerError;
use crate::context::ReplayerContext;
use race_event_record::{Record, RecordsHeader};
use tui::{
    style::{Color, Style},
    backend::CrosstermBackend,
    text::Text,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    layout::{Layout, Constraint, Direction},
    Terminal
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

struct RecordsViewState {
    pub(crate) records: Vec<Record>,
    pub(crate) state: ListState,
}

impl RecordsViewState {
    fn new(records: Vec<Record>) -> Self {
        Self { records, state: ListState::default() }
    }

    pub fn set_items(&mut self, records: Vec<Record>) {
        self.records = records;
        self.state = ListState::default();
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.records.len() - 1 {
                    0
                } else {
                    i + 1
                }
            },
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.records.len() - 1
                } else {
                    i - 1
                }
            },
            None => self.records.len() - 1,
        };
        self.state.select(Some(i));
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}

pub fn render_controller_ui(context: Arc<ReplayerContext>, header: RecordsHeader, records: Vec<Record>) -> Result<(), ReplayerError> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut records_view_state = RecordsViewState::new(records);

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(5),
                    Constraint::Min(1),
                ])
                .split(f.size());

            let header_spans = Text::from(format!("CHAIN: {}\nGAME: {}#{}\nBUNDLE: {}", header.chain, header.game_addr, header.game_id, header.bundle_addr));

            let paragraph = Paragraph::new(header_spans)
                .block(Block::default().title("Info").borders(Borders::ALL))
                .wrap(Wrap { trim: true });

            f.render_widget(paragraph, chunks[0]);

            let items: Vec<ListItem> = records_view_state.records.iter().map(|r| ListItem::new(format!("{}", r))).collect();

            let list = List::new(items)
                .block(Block::default().title("events").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::Green));

            f.render_stateful_widget(list, chunks[1], &mut records_view_state.state);
        })?;

        if let Event::Key(key) = event::read()? {

            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('n') => records_view_state.next(),
                KeyCode::Char('p') => records_view_state.previous(),
                _ => (),
            }
        }
    }

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
