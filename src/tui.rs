use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Terminal;
use std::io::{self, Stdout};

pub struct TuiSelection {
    pub selected: Vec<usize>,
    pub cancelled: bool,
}

pub fn run_multi_select(
    title: &str,
    header: Option<&str>,
    items: &[String],
    preselected: &[usize],
) -> anyhow::Result<TuiSelection> {
    if items.is_empty() {
        return Ok(TuiSelection {
            selected: Vec::new(),
            cancelled: false,
        });
    }
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run_loop(&mut terminal, title, header, items, preselected);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    title: &str,
    header: Option<&str>,
    items: &[String],
    preselected: &[usize],
) -> anyhow::Result<TuiSelection> {
    let mut state = SelectionState::new(items.len(), preselected);
    loop {
        terminal.draw(|frame| {
            let size = frame.size();
            let constraints = if header.is_some() {
                vec![
                    Constraint::Length(1),
                    Constraint::Min(2),
                    Constraint::Length(2),
                ]
            } else {
                vec![Constraint::Min(2), Constraint::Length(2)]
            };
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(constraints)
                .split(size);

            let mut list_index = 0;
            if let Some(text) = header {
                let header_widget =
                    Paragraph::new(Text::from(vec![Line::from(vec![Span::raw(text)])]))
                        .alignment(Alignment::Left);
                frame.render_widget(header_widget, layout[0]);
                list_index = 1;
            }

            let list_items: Vec<ListItem> = items
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    let checked = if state.selected.contains(&idx) {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    let content = Line::from(vec![Span::raw(format!("{} {}", checked, item))]);
                    ListItem::new(content)
                })
                .collect();

            let list = List::new(list_items)
                .block(Block::default().title(title).borders(Borders::ALL))
                .highlight_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("➤ ");

            frame.render_stateful_widget(list, layout[list_index], &mut state.list_state);

            let hint = Paragraph::new(Text::from(vec![Line::from(vec![Span::raw(
                "Space toggle  Enter apply  q/Esc quit",
            )])]))
            .alignment(Alignment::Right);
            frame.render_widget(hint, layout[list_index + 1]);
        })?;

        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key {
                    KeyEvent {
                        code: KeyCode::Char('q'),
                        ..
                    }
                    | KeyEvent {
                        code: KeyCode::Esc, ..
                    }
                    | KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    } => {
                        return Ok(TuiSelection {
                            selected: Vec::new(),
                            cancelled: true,
                        })
                    }
                    KeyEvent {
                        code: KeyCode::Up, ..
                    } => state.previous(),
                    KeyEvent {
                        code: KeyCode::Down,
                        ..
                    } => state.next(),
                    KeyEvent {
                        code: KeyCode::Char(' '),
                        ..
                    } => state.toggle(),
                    KeyEvent {
                        code: KeyCode::Enter,
                        ..
                    } => {
                        return Ok(TuiSelection {
                            selected: state.selected.clone(),
                            cancelled: false,
                        })
                    }
                    _ => {}
                }
            }
        }
    }
}

struct SelectionState {
    list_state: ratatui::widgets::ListState,
    selected: Vec<usize>,
    len: usize,
}

impl SelectionState {
    fn new(len: usize, preselected: &[usize]) -> Self {
        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(0));
        Self {
            list_state,
            selected: preselected.to_vec(),
            len,
        }
    }

    fn next(&mut self) {
        let idx = match self.list_state.selected() {
            Some(idx) => {
                if idx + 1 >= self.len {
                    0
                } else {
                    idx + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(idx));
    }

    fn previous(&mut self) {
        let idx = match self.list_state.selected() {
            Some(idx) => {
                if idx == 0 {
                    self.len - 1
                } else {
                    idx - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(idx));
    }

    fn toggle(&mut self) {
        if let Some(idx) = self.list_state.selected() {
            if let Some(pos) = self.selected.iter().position(|&item| item == idx) {
                self.selected.remove(pos);
            } else {
                self.selected.push(idx);
            }
        }
    }
}
