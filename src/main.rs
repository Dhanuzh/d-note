use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
    },
    Terminal,
};
use std::{
    fs,
    io::{self, Stdout},
    path::PathBuf,
};
use chrono::Local;

// ── Checkbox helpers ──────────────────────────────────────────────────────────

fn is_todo_unchecked(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("- [ ] ") || t == "- [ ]"
}

fn is_todo_checked(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("- [x] ") || t.starts_with("- [X] ") || t == "- [x]" || t == "- [X]"
}

fn is_todo(line: &str) -> bool {
    is_todo_unchecked(line) || is_todo_checked(line)
}

fn toggle_todo(line: &str) -> String {
    if is_todo_unchecked(line) {
        line.replacen("- [ ]", "- [x]", 1)
    } else if is_todo_checked(line) {
        line.replacen("- [x]", "- [ ]", 1)
            .replacen("- [X]", "- [ ]", 1)
    } else {
        line.to_string()
    }
}

fn todo_progress(body: &str) -> Option<(usize, usize)> {
    let total = body.lines().filter(|l| is_todo(l)).count();
    if total == 0 {
        return None;
    }
    let done = body.lines().filter(|l| is_todo_checked(l)).count();
    Some((done, total))
}

// ── Data ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Note {
    id: usize,
    title: String,
    body: String,
    created_at: String,
}

impl Note {
    fn new(id: usize, title: String, body: String) -> Self {
        Self {
            id,
            title,
            body,
            created_at: Local::now().format("%Y-%m-%d %H:%M").to_string(),
        }
    }
}

// ── Persistence ───────────────────────────────────────────────────────────────

fn notes_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("notes.md")
}

fn load_notes() -> Vec<Note> {
    let path = notes_path();
    if !path.exists() {
        return vec![];
    }
    let content = fs::read_to_string(&path).unwrap_or_default();
    parse_notes(&content)
}

fn parse_notes(content: &str) -> Vec<Note> {
    let mut notes = Vec::new();
    let mut id = 1;
    for block in content.split("\n---\n") {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }
        let mut lines = block.lines();
        let title_line = lines.next().unwrap_or("").trim();
        let title = title_line.trim_start_matches("## ").to_string();
        if title.is_empty() || title.starts_with('#') {
            continue;
        }
        let mut created_at = String::new();
        let mut body_lines: Vec<&str> = Vec::new();
        let mut past_meta = false;
        for line in lines {
            if !past_meta && line.starts_with('_') && line.ends_with('_') {
                created_at = line.trim_matches('_').to_string();
                past_meta = true;
            } else if line.is_empty() && !past_meta {
                past_meta = true;
            } else {
                body_lines.push(line);
            }
        }
        let body = body_lines.join("\n").trim().to_string();
        notes.push(Note { id, title, body, created_at });
        id += 1;
    }
    notes
}

fn save_notes(notes: &[Note]) {
    let path = notes_path();
    let mut content = String::from("# Sticky Notes\n\n");
    for note in notes {
        content.push_str(&format!("## {}\n", note.title));
        content.push_str(&format!("_{}_\n\n", note.created_at));
        content.push_str(&note.body);
        content.push_str("\n\n---\n\n");
    }
    let _ = fs::write(path, content);
}

// ── App State ─────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone)]
enum Mode {
    List,
    View,
    AddTitle,
    AddBody,
    EditTitle,
    EditBody,
    ConfirmDelete,
    Help,
    Hidden,
}

struct App {
    notes: Vec<Note>,
    list_state: ListState,
    mode: Mode,
    prev_mode: Mode,
    input: String,
    temp_title: String,
    temp_body: String,
    status_msg: String,
    // view mode: which body line the cursor is on
    view_line: usize,
}

impl App {
    fn new() -> Self {
        let notes = load_notes();
        let mut list_state = ListState::default();
        if !notes.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            notes,
            list_state,
            mode: Mode::List,
            prev_mode: Mode::List,
            input: String::new(),
            temp_title: String::new(),
            temp_body: String::new(),
            status_msg: String::new(),
            view_line: 0,
        }
    }

    fn selected(&self) -> Option<&Note> {
        self.list_state.selected().and_then(|i| self.notes.get(i))
    }

    fn next_id(&self) -> usize {
        self.notes.iter().map(|n| n.id).max().unwrap_or(0) + 1
    }

    fn delete_selected(&mut self) {
        if let Some(i) = self.list_state.selected() {
            self.notes.remove(i);
            save_notes(&self.notes);
            let new_sel = if self.notes.is_empty() {
                None
            } else {
                Some(i.min(self.notes.len() - 1))
            };
            self.list_state.select(new_sel);
            self.status_msg = "Note deleted.".into();
        }
        self.mode = Mode::List;
    }

    fn add_note(&mut self) {
        let title = self.temp_title.trim().to_string();
        let body = self.temp_body.trim().to_string();
        if title.is_empty() {
            self.status_msg = "Title cannot be empty.".into();
            self.mode = Mode::List;
            return;
        }
        let id = self.next_id();
        self.notes.push(Note::new(id, title, body));
        save_notes(&self.notes);
        self.list_state.select(Some(self.notes.len() - 1));
        self.status_msg = "Note added.".into();
        self.mode = Mode::List;
    }

    fn save_edit(&mut self) {
        if let Some(i) = self.list_state.selected() {
            let title = self.temp_title.trim().to_string();
            if title.is_empty() {
                self.status_msg = "Title cannot be empty.".into();
                self.mode = Mode::List;
                return;
            }
            self.notes[i].title = title;
            self.notes[i].body = self.temp_body.trim().to_string();
            save_notes(&self.notes);
            self.status_msg = "Note updated.".into();
        }
        self.mode = Mode::List;
    }

    fn toggle_view_line(&mut self) {
        if let Some(i) = self.list_state.selected() {
            let lines: Vec<String> =
                self.notes[i].body.lines().map(|l| l.to_string()).collect();
            if self.view_line < lines.len() && is_todo(&lines[self.view_line]) {
                let new_lines: Vec<String> = lines
                    .iter()
                    .enumerate()
                    .map(|(li, l)| {
                        if li == self.view_line {
                            toggle_todo(l)
                        } else {
                            l.clone()
                        }
                    })
                    .collect();
                self.notes[i].body = new_lines.join("\n");
                save_notes(&self.notes);
            }
        }
    }
}

// ── Layout ────────────────────────────────────────────────────────────────────

fn sticky_rect(area: Rect) -> Rect {
    let width: u16 = 50;
    let height: u16 = 32;
    let x = area.width.saturating_sub(width);
    Rect::new(x, 0, width.min(area.width), height.min(area.height))
}

// ── Styles (terminal-theme-aware) ─────────────────────────────────────────────

fn style_base() -> Style {
    Style::default().fg(Color::Reset).bg(Color::Reset)
}
fn style_border() -> Style {
    Style::default().fg(Color::Cyan)
}
fn style_title() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}
fn style_dim() -> Style {
    Style::default().fg(Color::DarkGray)
}
fn style_selected() -> Style {
    Style::default().add_modifier(Modifier::REVERSED).add_modifier(Modifier::BOLD)
}
fn style_checked() -> Style {
    Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::DIM)
}
fn style_unchecked() -> Style {
    Style::default().fg(Color::Reset)
}
fn style_highlight_line() -> Style {
    Style::default().add_modifier(Modifier::REVERSED)
}
fn style_key() -> Style {
    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
}
fn style_input_active() -> Style {
    Style::default().fg(Color::Blue)
}
fn style_success() -> Style {
    Style::default().fg(Color::Green)
}
fn style_error() -> Style {
    Style::default().fg(Color::Red)
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn ui(f: &mut ratatui::Frame, app: &App) {
    if app.mode == Mode::Hidden {
        let area = f.area();
        // tiny pill hint in top-right
        let hint_rect = Rect::new(area.width.saturating_sub(24), 0, 24, 1);
        f.render_widget(
            Paragraph::new(Span::styled(
                " notes  ^Space ",
                style_dim(),
            )),
            hint_rect,
        );
        return;
    }

    let area = f.area();
    let rect = sticky_rect(area);
    f.render_widget(Clear, rect);

    // outer panel
    let note_count = app.notes.len();
    let panel_title = format!(" notes ({note_count}) ");
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(panel_title.as_str(), style_title()))
        .title_alignment(Alignment::Center)
        .border_style(style_border())
        .style(style_base());

    let inner = outer.inner(rect);
    f.render_widget(outer, rect);

    match &app.mode {
        Mode::List => render_list(f, app, inner),
        Mode::View => render_view(f, app, inner),
        Mode::AddTitle | Mode::EditTitle => render_input_title(f, app, inner),
        Mode::AddBody | Mode::EditBody => render_input_body(f, app, inner),
        Mode::ConfirmDelete => render_confirm(f, app, inner),
        Mode::Help => render_help(f, inner),
        Mode::Hidden => {}
    }
}

fn render_list(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    if app.notes.is_empty() {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No notes yet.",
                    style_dim(),
                )),
                Line::from(Span::styled(
                    "  Press a to create one.",
                    style_dim(),
                )),
            ])
            .style(style_base()),
            chunks[0],
        );
    } else {
        let items: Vec<ListItem> = app
            .notes
            .iter()
            .map(|n| {
                let mut spans = vec![];

                // todo progress badge
                if let Some((done, total)) = todo_progress(&n.body) {
                    let badge = format!("[{done}/{total}] ");
                    let badge_style = if done == total {
                        style_checked()
                    } else {
                        Style::default().fg(Color::Yellow)
                    };
                    spans.push(Span::styled(badge, badge_style));
                }

                // title
                let max_title = (area.width as usize).saturating_sub(
                    spans.iter().map(|s| s.content.len()).sum::<usize>() + 2,
                );
                let title = if n.title.len() > max_title {
                    format!("{}…", &n.title[..max_title.saturating_sub(1)])
                } else {
                    n.title.clone()
                };
                spans.push(Span::raw(title));

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .style(style_base())
            .highlight_style(style_selected())
            .highlight_symbol(" > ");

        f.render_stateful_widget(list, chunks[0], &mut app.list_state.clone());
    }

    // footer hints
    let footer = if !app.status_msg.is_empty() {
        let sty = if app.status_msg.contains("deleted") || app.status_msg.contains("empty") {
            style_error()
        } else {
            style_success()
        };
        Line::from(Span::styled(app.status_msg.as_str(), sty))
    } else {
        Line::from(vec![
            Span::styled("a", style_key()),
            Span::styled("dd ", style_dim()),
            Span::styled("e", style_key()),
            Span::styled("dit ", style_dim()),
            Span::styled("d", style_key()),
            Span::styled("el ", style_dim()),
            Span::styled("↵", style_key()),
            Span::styled("view ", style_dim()),
            Span::styled("?", style_key()),
            Span::styled("help ", style_dim()),
            Span::styled("h", style_key()),
            Span::styled("ide", style_dim()),
        ])
    };
    f.render_widget(Paragraph::new(footer), chunks[1]);
}

fn render_view(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let Some(note) = app.selected() else { return };

    let body_lines: Vec<&str> = note.body.lines().collect();
    let has_todos = body_lines.iter().any(|l| is_todo(l));

    // header: title + date
    let header_height = 2u16;
    let footer_height = 1u16;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Min(1),
            Constraint::Length(footer_height),
        ])
        .split(area);

    // title row with optional progress
    let mut title_spans = vec![Span::styled(
        note.title.as_str(),
        Style::default().fg(Color::Reset).add_modifier(Modifier::BOLD),
    )];
    if let Some((done, total)) = todo_progress(&note.body) {
        let prog = format!("  {done}/{total}");
        let sty = if done == total { style_checked() } else { Style::default().fg(Color::Yellow) };
        title_spans.push(Span::styled(prog, sty));
    }

    f.render_widget(
        Paragraph::new(vec![
            Line::from(title_spans),
            Line::from(Span::styled(note.created_at.as_str(), style_dim())),
        ]),
        chunks[0],
    );

    // body lines
    let rendered_lines: Vec<Line> = body_lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let is_cursor = i == app.view_line;
            if is_todo_checked(line) {
                let text = line.trim_start_matches("- [x] ").trim_start_matches("- [X] ");
                let spans = vec![
                    Span::styled(" [x] ", style_checked()),
                    Span::styled(text, style_checked()),
                ];
                if is_cursor {
                    Line::from(spans).style(style_highlight_line())
                } else {
                    Line::from(spans)
                }
            } else if is_todo_unchecked(line) {
                let text = line.trim_start_matches("- [ ] ");
                let spans = vec![
                    Span::styled(" [ ] ", style_unchecked()),
                    Span::raw(text),
                ];
                if is_cursor {
                    Line::from(spans).style(style_highlight_line())
                } else {
                    Line::from(spans)
                }
            } else {
                let l = Line::from(Span::raw(*line));
                if is_cursor && has_todos {
                    l.style(style_highlight_line())
                } else {
                    l
                }
            }
        })
        .collect();

    f.render_widget(
        Paragraph::new(rendered_lines).wrap(Wrap { trim: false }),
        chunks[1],
    );

    // footer
    let footer = if has_todos {
        Line::from(vec![
            Span::styled("↑↓", style_key()),
            Span::styled(" nav  ", style_dim()),
            Span::styled("Space", style_key()),
            Span::styled(" toggle  ", style_dim()),
            Span::styled("e", style_key()),
            Span::styled("dit  ", style_dim()),
            Span::styled("Esc", style_key()),
            Span::styled(" back", style_dim()),
        ])
    } else {
        Line::from(vec![
            Span::styled("Esc", style_key()),
            Span::styled(" back  ", style_dim()),
            Span::styled("e", style_key()),
            Span::styled("dit  ", style_dim()),
            Span::styled("d", style_key()),
            Span::styled("el", style_dim()),
        ])
    };
    f.render_widget(Paragraph::new(footer), chunks[2]);
}

fn render_input_title(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let editing = app.mode == Mode::EditTitle;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let label = if editing { " Edit title " } else { " New note — title " };
    let blk = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(label, style_input_active()))
        .border_style(style_input_active());
    let inner = blk.inner(chunks[0]);
    f.render_widget(blk, chunks[0]);
    // scroll text left so cursor is always visible
    let w = inner.width as usize;
    let visible = if app.input.len() >= w {
        &app.input[app.input.len() + 1 - w..]
    } else {
        &app.input
    };
    f.render_widget(Paragraph::new(visible), inner);
    f.set_cursor_position((inner.x + visible.len().min(w) as u16, inner.y));

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Enter", style_key()),
            Span::styled(" next    ", style_dim()),
            Span::styled("Esc", style_key()),
            Span::styled(" cancel", style_dim()),
        ])),
        chunks[1],
    );
}

fn render_input_body(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let editing = app.mode == Mode::EditBody;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(area);

    let label = if editing { " Edit body " } else { " New note — body " };
    let blk = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(label, style_input_active()))
        .border_style(style_input_active());
    let inner = blk.inner(chunks[0]);
    f.render_widget(blk, chunks[0]);
    f.render_widget(
        Paragraph::new(app.input.as_str()).wrap(Wrap { trim: false }),
        inner,
    );
    let col = app.input.lines().last().map(|l| l.len()).unwrap_or(0) as u16;
    let row = (app.input.lines().count() as u16).saturating_sub(1);
    f.set_cursor_position((inner.x + col, inner.y + row));

    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Ctrl+S", style_key()),
                Span::styled(" save    ", style_dim()),
                Span::styled("Esc", style_key()),
                Span::styled(" cancel", style_dim()),
            ]),
            Line::from(Span::styled(
                "Tip: - [ ] task  or  - [x] done",
                style_dim(),
            )),
        ]),
        chunks[1],
    );
}

fn render_confirm(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let title = app.selected().map(|n| n.title.as_str()).unwrap_or("?");
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  Delete \"{}\"?", title),
                style_error().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  y", style_key()),
                Span::styled(" yes    ", style_dim()),
                Span::styled("n", style_key()),
                Span::styled(" / ", style_dim()),
                Span::styled("Esc", style_key()),
                Span::styled(" no", style_dim()),
            ]),
        ])
        .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_help(f: &mut ratatui::Frame, area: Rect) {
    let k = style_key();
    let d = style_dim();
    let h = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let help: Vec<Line> = vec![
        Line::from(Span::styled(" Shortcuts", h)),
        Line::from(""),
        Line::from(Span::styled(" List", h)),
        Line::from(vec![Span::styled("  a", k), Span::styled("   add note", d)]),
        Line::from(vec![Span::styled("  e", k), Span::styled("   edit", d)]),
        Line::from(vec![Span::styled("  d", k), Span::styled("   delete", d)]),
        Line::from(vec![Span::styled("  ↵", k), Span::styled("   view note", d)]),
        Line::from(vec![Span::styled("  j/k ↑↓", k), Span::styled("  navigate", d)]),
        Line::from(vec![Span::styled("  h", k), Span::styled("   hide panel", d)]),
        Line::from(vec![Span::styled("  q", k), Span::styled("   quit", d)]),
        Line::from(""),
        Line::from(Span::styled(" View", h)),
        Line::from(vec![Span::styled("  Space", k), Span::styled(" toggle checkbox", d)]),
        Line::from(vec![Span::styled("  e", k), Span::styled("     edit note", d)]),
        Line::from(vec![Span::styled("  Esc", k), Span::styled("   back", d)]),
        Line::from(""),
        Line::from(Span::styled(" Edit", h)),
        Line::from(vec![Span::styled("  Ctrl+S", k), Span::styled(" save", d)]),
        Line::from(vec![Span::styled("  Esc", k), Span::styled("    cancel", d)]),
        Line::from(""),
        Line::from(Span::styled(" Anywhere", h)),
        Line::from(vec![Span::styled("  ^Space", k), Span::styled(" show/hide", d)]),
        Line::from(""),
        Line::from(Span::styled(" Checkboxes in body:", h)),
        Line::from(Span::styled("  - [ ] pending", d)),
        Line::from(Span::styled("  - [x] done", d)),
    ];
    f.render_widget(Paragraph::new(help).wrap(Wrap { trim: false }), area);
}

// ── Event Handling ────────────────────────────────────────────────────────────

fn handle_events(app: &mut App) -> io::Result<bool> {
    if !event::poll(std::time::Duration::from_millis(100))? {
        return Ok(false);
    }
    if let Event::Key(key) = event::read()? {
        // Ctrl+Space: global show/hide
        if key.code == KeyCode::Char(' ') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if app.mode == Mode::Hidden {
                app.mode = app.prev_mode.clone();
            } else {
                app.prev_mode = app.mode.clone();
                app.mode = Mode::Hidden;
            }
            return Ok(false);
        }

        match &app.mode {
            Mode::Hidden => {}

            Mode::List => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
                KeyCode::Char('?') => app.mode = Mode::Help,
                KeyCode::Char('h') => {
                    app.prev_mode = Mode::List;
                    app.mode = Mode::Hidden;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if !app.notes.is_empty() {
                        let i = app.list_state.selected().unwrap_or(0);
                        app.list_state
                            .select(Some(if i == 0 { app.notes.len() - 1 } else { i - 1 }));
                    }
                    app.status_msg.clear();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !app.notes.is_empty() {
                        let i = app.list_state.selected().unwrap_or(0);
                        app.list_state.select(Some((i + 1) % app.notes.len()));
                    }
                    app.status_msg.clear();
                }
                KeyCode::Enter => {
                    if app.selected().is_some() {
                        app.view_line = 0;
                        app.mode = Mode::View;
                    }
                }
                KeyCode::Char('a') => {
                    app.temp_title.clear();
                    app.temp_body.clear();
                    app.input.clear();
                    app.mode = Mode::AddTitle;
                }
                KeyCode::Char('e') => {
                    let data = app.selected().map(|n| (n.title.clone(), n.body.clone()));
                    if let Some((title, body)) = data {
                        app.temp_title = title.clone();
                        app.temp_body = body;
                        app.input = title;
                        app.mode = Mode::EditTitle;
                    }
                }
                KeyCode::Char('d') => {
                    if app.selected().is_some() {
                        app.mode = Mode::ConfirmDelete;
                    }
                }
                _ => {}
            },

            Mode::View => {
                let body_len = app
                    .selected()
                    .map(|n| n.body.lines().count())
                    .unwrap_or(0);
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => app.mode = Mode::List,
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.view_line = app.view_line.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if body_len > 0 && app.view_line < body_len - 1 {
                            app.view_line += 1;
                        }
                    }
                    KeyCode::Char(' ') => {
                        app.toggle_view_line();
                    }
                    KeyCode::Char('e') => {
                        let data = app.selected().map(|n| (n.title.clone(), n.body.clone()));
                        if let Some((title, body)) = data {
                            app.temp_title = title.clone();
                            app.temp_body = body;
                            app.input = title;
                            app.mode = Mode::EditTitle;
                        }
                    }
                    KeyCode::Char('d') => {
                        if app.selected().is_some() {
                            app.mode = Mode::ConfirmDelete;
                        }
                    }
                    _ => {}
                }
            }

            Mode::AddTitle | Mode::EditTitle => match key.code {
                KeyCode::Esc => {
                    app.mode = Mode::List;
                    app.input.clear();
                }
                KeyCode::Enter => {
                    if app.mode == Mode::AddTitle {
                        app.temp_title = app.input.trim().to_string();
                        app.input.clear();
                        app.mode = Mode::AddBody;
                    } else {
                        app.temp_title = app.input.trim().to_string();
                        app.input = app.temp_body.clone();
                        app.mode = Mode::EditBody;
                    }
                }
                KeyCode::Backspace => {
                    app.input.pop();
                }
                KeyCode::Char(c) => app.input.push(c),
                _ => {}
            },

            Mode::AddBody | Mode::EditBody => {
                if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    if app.mode == Mode::AddBody {
                        app.temp_body = app.input.clone();
                        app.input.clear();
                        app.add_note();
                    } else {
                        app.temp_body = app.input.clone();
                        app.input.clear();
                        app.save_edit();
                    }
                } else {
                    match key.code {
                        KeyCode::Esc => {
                            app.mode = Mode::List;
                            app.input.clear();
                        }
                        KeyCode::Enter => app.input.push('\n'),
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        KeyCode::Char(c) => app.input.push(c),
                        _ => {}
                    }
                }
            }

            Mode::ConfirmDelete => match key.code {
                KeyCode::Char('y') => app.delete_selected(),
                KeyCode::Char('n') | KeyCode::Esc => app.mode = Mode::List,
                _ => {}
            },

            Mode::Help => match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                    app.mode = Mode::List;
                }
                _ => {}
            },
        }
    }
    Ok(false)
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new();
    let result = run(&mut terminal, &mut app);
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;
        if handle_events(app)? {
            break;
        }
    }
    Ok(())
}
