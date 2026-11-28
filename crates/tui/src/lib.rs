#![forbid(unsafe_code)]
//! Antarmuka terminal (`ratatui`) klien tipis yang berkomunikasi ke daemon lewat UDS (T-140..T-142).

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use latch_core::id::short_id;
use latch_core::model::{
    MergeStrategy, PipelineRun, PipelineStatus, Session, SessionDiff, SessionStatus,
};
use latch_core::protocol::{
    read_raw_frame, write_raw_frame, AttachFrame, IpcReadHalf, IpcWriteHalf, PipelineListParams,
    SessionDiffParams, SessionListParams, SessionMergeParams, UdsClient,
};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use thiserror::Error;

/// Kesalahan pada antarmuka TUI.
#[derive(Debug, Error)]
pub enum TuiError {
    /// Galat I/O terminal.
    #[error("terminal I/O error: {0}")]
    Io(#[from] io::Error),
    /// Galat protokol komunikasi ke daemon.
    #[error("daemon protocol error: {0}")]
    Protocol(#[from] latch_core::protocol::ProtocolError),
}

/// Mode tampilan aktif pada TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiMode {
    /// Jelajah daftar sesi dan pratinjau snapshot layar.
    Browse,
    /// Terhubung langsung (interactive attach) ke PTY sesi.
    Attach { session_id: String },
    /// Menampilkan diff worktree sesi terpilih.
    Diff { session_id: String },
    /// Menampilkan daftar pipeline DAG dan matriks perbandingan Fan-Out Arena (T-409).
    Pipelines,
    /// Dialog konfirmasi aksi destruktif/final.
    Confirm {
        action: ConfirmAction,
        session_id: String,
    },
    /// Bantuan pintasan keyboard.
    Help,
}

/// Aksi yang memerlukan konfirmasi pengguna.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmAction {
    /// Hentikan sesi paksa.
    Kill,
    /// Squash merge ke branch default.
    Merge,
    /// Buang sesi beserta worktree.
    Discard,
}

/// State aplikasi TUI (`latch-tui`).
pub struct TuiApp {
    /// Daftar sesi terbaru dari daemon.
    pub sessions: Vec<Session>,
    /// Indeks sesi terpilih.
    pub selected: usize,
    /// Daftar pipeline terbaru dari daemon (`T-409`).
    pub pipelines: Vec<PipelineRun>,
    /// Indeks pipeline terpilih.
    pub selected_pipeline: usize,
    /// Mode UI aktif.
    pub mode: UiMode,
    /// Diff sesi yang terakhir dimuat.
    pub current_diff: Option<SessionDiff>,
    /// Parser terminal virtual untuk menampilkan layar/warna ANSI sesi terpilih.
    pub vt: vt100::Parser,
    /// Pesan status di baris bawah.
    pub status_line: String,
    /// Status prefix `Ctrl-b` untuk detach (`Ctrl-b d`).
    pub ctrl_b_armed: bool,
    /// Penanda keluar dari loop TUI.
    pub should_quit: bool,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TuiApp {
    /// Membangun state TUI awal.
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            selected: 0,
            pipelines: Vec::new(),
            selected_pipeline: 0,
            mode: UiMode::Browse,
            current_diff: None,
            vt: vt100::Parser::new(24, 80, 500),
            status_line: "Siap. Tekan ? untuk bantuan, q untuk keluar.".to_string(),
            ctrl_b_armed: false,
            should_quit: false,
        }
    }

    /// Mengambil sesi yang sedang dipilih.
    pub fn selected_session(&self) -> Option<&Session> {
        self.sessions.get(self.selected)
    }

    /// Memindah kursor seleksi sesi ke bawah.
    pub fn select_next(&mut self) {
        if !self.sessions.is_empty() {
            self.selected = (self.selected + 1) % self.sessions.len();
        }
    }

    /// Memindah kursor seleksi sesi ke atas.
    pub fn select_prev(&mut self) {
        if !self.sessions.is_empty() {
            self.selected = if self.selected == 0 {
                self.sessions.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    /// Memperbarui daftar sesi dengan menjaga posisi pilihan tetap valid.
    pub fn set_sessions(&mut self, sessions: Vec<Session>) {
        self.sessions = sessions;
        if self.sessions.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.sessions.len() {
            self.selected = self.sessions.len() - 1;
        }
    }

    /// Mengambil pipeline yang sedang dipilih.
    pub fn selected_pipeline(&self) -> Option<&PipelineRun> {
        self.pipelines.get(self.selected_pipeline)
    }

    /// Memperbarui daftar pipeline dengan menjaga indeks seleksi tetap valid.
    pub fn set_pipelines(&mut self, pipelines: Vec<PipelineRun>) {
        self.pipelines = pipelines;
        if self.pipelines.is_empty() {
            self.selected_pipeline = 0;
        } else if self.selected_pipeline >= self.pipelines.len() {
            self.selected_pipeline = self.pipelines.len() - 1;
        }
    }
}

struct AttachConn {
    reader: IpcReadHalf,
    writer: IpcWriteHalf,
}

/// Menjalankan antarmuka TUI interaktif yang tersambung ke UDS daemon.
pub async fn run_tui(socket_path: PathBuf) -> Result<(), TuiError> {
    let client = UdsClient::new(socket_path);
    let mut app = TuiApp::new();
    if let Ok(list) = client.session_list(&SessionListParams::default()).await {
        app.set_sessions(list);
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_event_loop(&mut terminal, &client, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    res
}

async fn run_event_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    client: &UdsClient,
    app: &mut TuiApp,
) -> Result<(), TuiError> {
    let mut attach_conn: Option<AttachConn> = None;
    let mut refresh_tick = tokio::time::interval(Duration::from_millis(750));

    while !app.should_quit {
        terminal.draw(|frame| render_ui(frame, app))?;

        // Baca frame PTY jika sedang dalam mode attach
        if let Some(conn) = attach_conn.as_mut() {
            while let Ok(res) =
                tokio::time::timeout(Duration::from_millis(5), read_raw_frame(&mut conn.reader))
                    .await
            {
                match res {
                    Ok(Some(raw)) => {
                        if let Ok(frame) = AttachFrame::decode(&raw) {
                            match frame {
                                AttachFrame::Snapshot(bytes) | AttachFrame::Output(bytes) => {
                                    app.vt.process(&bytes);
                                }
                                AttachFrame::Dropped(n) => {
                                    app.status_line = format!("Peringatan: {n} byte terlewati");
                                }
                                AttachFrame::Detach => {
                                    attach_conn = None;
                                    app.mode = UiMode::Browse;
                                    app.status_line = "Detached dari sesi.".to_string();
                                    break;
                                }
                                AttachFrame::Input(_) | AttachFrame::Resize { .. } => {}
                            }
                        }
                    }
                    Ok(None) | Err(_) => {
                        attach_conn = None;
                        app.mode = UiMode::Browse;
                        app.status_line = "Koneksi PTY berakhir.".to_string();
                        break;
                    }
                }
            }
        }

        // Poll input terminal non-blocking singkat
        while event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Resize(cols, rows) => {
                    let right_cols = cols
                        .saturating_sub(cols * 36 / 100)
                        .saturating_sub(2)
                        .max(20);
                    let right_rows = rows.saturating_sub(5).max(5);
                    app.vt.set_size(right_rows, right_cols);
                    if let Some(conn) = attach_conn.as_mut() {
                        let _ = write_raw_frame(
                            &mut conn.writer,
                            &AttachFrame::Resize {
                                cols: right_cols,
                                rows: right_rows,
                            }
                            .encode(),
                        )
                        .await;
                    }
                }
                Event::Key(key) => {
                    handle_key(key, client, app, &mut attach_conn).await;
                }
                _ => {}
            }
        }

        if tokio::time::timeout(Duration::from_millis(1), refresh_tick.tick())
            .await
            .is_ok()
        {
            if let Ok(list) = client.session_list(&SessionListParams::default()).await {
                app.set_sessions(list);
            }
            if let Ok(pipes) = client.pipeline_list(&PipelineListParams::default()).await {
                app.set_pipelines(pipes);
            }
            if matches!(app.mode, UiMode::Browse) {
                if let Some(ses) = app.selected_session().cloned() {
                    if let Ok(snap) = client.session_screen(&ses.id).await {
                        app.vt = vt100::Parser::new(snap.rows.max(5), snap.cols.max(20), 500);
                        app.vt.process(&snap.ansi_bytes);
                    }
                }
            }
        }
    }

    if let Some(mut conn) = attach_conn {
        let _ = write_raw_frame(&mut conn.writer, &AttachFrame::Detach.encode()).await;
    }
    Ok(())
}

async fn handle_key(
    key: KeyEvent,
    client: &UdsClient,
    app: &mut TuiApp,
    attach_conn: &mut Option<AttachConn>,
) {
    match app.mode.clone() {
        UiMode::Attach { .. } => {
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b') {
                app.ctrl_b_armed = true;
                app.status_line = "Ctrl-b aktif: tekan 'd' untuk detach.".to_string();
                return;
            }
            if app.ctrl_b_armed {
                app.ctrl_b_armed = false;
                if key.code == KeyCode::Char('d') {
                    if let Some(mut conn) = attach_conn.take() {
                        let _ =
                            write_raw_frame(&mut conn.writer, &AttachFrame::Detach.encode()).await;
                    }
                    app.mode = UiMode::Browse;
                    app.status_line = "Detached dari PTY.".to_string();
                    return;
                }
            }
            if let Some(conn) = attach_conn.as_mut() {
                if let Some(bytes) = encode_key_bytes(key) {
                    let _ = write_raw_frame(&mut conn.writer, &AttachFrame::Input(bytes).encode())
                        .await;
                }
            }
        }
        UiMode::Help => {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
            ) {
                app.mode = UiMode::Browse;
            }
        }
        UiMode::Diff { .. } => {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('d')
            ) {
                app.mode = UiMode::Browse;
            }
        }
        UiMode::Pipelines => match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('P') => {
                app.mode = UiMode::Browse;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !app.pipelines.is_empty() {
                    app.selected_pipeline = (app.selected_pipeline + 1) % app.pipelines.len();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !app.pipelines.is_empty() {
                    app.selected_pipeline = if app.selected_pipeline == 0 {
                        app.pipelines.len() - 1
                    } else {
                        app.selected_pipeline - 1
                    };
                }
            }
            KeyCode::Char('c') => {
                if let Some(run) = app.selected_pipeline().cloned() {
                    match client.pipeline_cancel(&run.id).await {
                        Ok(u) => {
                            app.status_line =
                                format!("Pipeline {} -> {}", short_id(&u.id), u.status.as_str());
                        }
                        Err(e) => app.status_line = format!("Gagal batalkan pipeline: {e}"),
                    }
                    if let Ok(pipes) = client.pipeline_list(&PipelineListParams::default()).await {
                        app.set_pipelines(pipes);
                    }
                }
            }
            _ => {}
        },
        UiMode::Confirm { action, session_id } => match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                match action {
                    ConfirmAction::Kill => match client.session_kill(&session_id).await {
                        Ok(_) => app.status_line = "Sesi dihentikan.".to_string(),
                        Err(e) => app.status_line = format!("Gagal kill: {e}"),
                    },
                    ConfirmAction::Merge => {
                        match client
                            .session_merge(&SessionMergeParams {
                                session_id,
                                strategy: MergeStrategy::Squash,
                                message: None,
                            })
                            .await
                        {
                            Ok(_) => app.status_line = "Squash merge berhasil.".to_string(),
                            Err(e) => app.status_line = format!("Gagal merge: {e}"),
                        }
                    }
                    ConfirmAction::Discard => match client.session_discard(&session_id).await {
                        Ok(()) => app.status_line = "Sesi & worktree dibuang.".to_string(),
                        Err(e) => app.status_line = format!("Gagal discard: {e}"),
                    },
                }
                if let Ok(list) = client.session_list(&SessionListParams::default()).await {
                    app.set_sessions(list);
                }
                app.mode = UiMode::Browse;
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                app.mode = UiMode::Browse;
                app.status_line = "Aksi dibatalkan.".to_string();
            }
            _ => {}
        },
        UiMode::Browse => match key.code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('?') => app.mode = UiMode::Help,
            KeyCode::Down | KeyCode::Char('j') => app.select_next(),
            KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
            KeyCode::Enter | KeyCode::Char('a') => {
                if let Some(ses) = app.selected_session().cloned() {
                    match client.attach_stream(&ses.id).await {
                        Ok(stream) => {
                            let (reader, writer) = stream.into_split();
                            *attach_conn = Some(AttachConn { reader, writer });
                            app.vt = vt100::Parser::new(24, 80, 500);
                            app.mode = UiMode::Attach { session_id: ses.id };
                            app.status_line =
                                "Attached. Tekan Ctrl-b lalu d untuk detach.".to_string();
                        }
                        Err(e) => app.status_line = format!("Gagal attach: {e}"),
                    }
                }
            }
            KeyCode::Char('p') => {
                if let Some(ses) = app.selected_session().cloned() {
                    let res = if ses.status == SessionStatus::Paused {
                        client.session_resume(&ses.id).await
                    } else {
                        client.session_pause(&ses.id).await
                    };
                    match res {
                        Ok(u) => app.status_line = format!("Status sesi -> {}", u.status),
                        Err(e) => app.status_line = format!("Gagal pause/resume: {e}"),
                    }
                    if let Ok(list) = client.session_list(&SessionListParams::default()).await {
                        app.set_sessions(list);
                    }
                }
            }
            KeyCode::Char('d') => {
                if let Some(ses) = app.selected_session().cloned() {
                    match client
                        .session_diff(&SessionDiffParams {
                            session_id: ses.id.clone(),
                            file: None,
                            max_bytes: Some(128 * 1024),
                        })
                        .await
                    {
                        Ok(diff) => {
                            app.current_diff = Some(diff);
                            app.mode = UiMode::Diff { session_id: ses.id };
                        }
                        Err(e) => app.status_line = format!("Gagal memuat diff: {e}"),
                    }
                }
            }
            KeyCode::Char('x') => {
                if let Some(ses) = app.selected_session() {
                    app.mode = UiMode::Confirm {
                        action: ConfirmAction::Kill,
                        session_id: ses.id.clone(),
                    };
                }
            }
            KeyCode::Char('m') => {
                if let Some(ses) = app.selected_session() {
                    app.mode = UiMode::Confirm {
                        action: ConfirmAction::Merge,
                        session_id: ses.id.clone(),
                    };
                }
            }
            KeyCode::Char('X') => {
                if let Some(ses) = app.selected_session() {
                    app.mode = UiMode::Confirm {
                        action: ConfirmAction::Discard,
                        session_id: ses.id.clone(),
                    };
                }
            }
            KeyCode::Char('P') => {
                if let Ok(pipes) = client.pipeline_list(&PipelineListParams::default()).await {
                    app.set_pipelines(pipes);
                }
                app.mode = UiMode::Pipelines;
            }
            _ => {}
        },
    }
}

/// Merender seluruh tata letak TUI Latch (`ratatui`).
pub fn render_ui(frame: &mut Frame<'_>, app: &mut TuiApp) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " LATCH TUI ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            "  Sesi: {}  |  Mode: {:?}",
            app.sessions.len(),
            app.mode
        )),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Agent Orchestrator"),
    );
    frame.render_widget(header, outer[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(36), Constraint::Percentage(64)])
        .split(outer[1]);

    if matches!(app.mode, UiMode::Pipelines) {
        render_pipelines_list(frame, app, body[0]);
        render_pipeline_arena_panel(frame, app, body[1]);
    } else {
        render_session_list(frame, app, body[0]);
        match &app.mode {
            UiMode::Diff { .. } => render_diff_panel(frame, app, body[1]),
            _ => render_pty_panel(frame, app, body[1]),
        }
    }

    let footer = Paragraph::new(vec![
        Line::from(app.status_line.as_str()),
        Line::from("↑/↓ j/k: pilih | Enter/a: attach | Ctrl-b d: detach | p: pause/resume | d: diff | P: pipeline/arena | m: merge | x: kill | X: discard | ?: bantuan | q: keluar"),
    ])
    .style(Style::default().fg(Color::Gray));
    frame.render_widget(footer, outer[2]);

    match &app.mode {
        UiMode::Confirm { action, session_id } => render_confirm_modal(frame, *action, session_id),
        UiMode::Help => render_help_modal(frame),
        _ => {}
    }
}

fn render_session_list(frame: &mut Frame<'_>, app: &TuiApp, area: Rect) {
    let items: Vec<ListItem<'_>> = app
        .sessions
        .iter()
        .map(|s| {
            let color = match s.status {
                SessionStatus::Running => Color::Green,
                SessionStatus::Paused => Color::Yellow,
                SessionStatus::Completed => Color::Cyan,
                SessionStatus::Tripped | SessionStatus::Failed => Color::Red,
                SessionStatus::WaitingApproval => Color::Magenta,
                SessionStatus::Hibernated => Color::Blue,
            };
            let line = Line::from(vec![
                Span::styled(format!("[{}] ", s.status), Style::default().fg(color)),
                Span::styled(
                    short_id(&s.id),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(" {} ($0.00)", s.agent_profile)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    if !app.sessions.is_empty() {
        state.select(Some(app.selected));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Sesi"))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_pty_panel(frame: &mut Frame<'_>, app: &TuiApp, area: Rect) {
    let screen = app.vt.screen();
    let (rows, cols) = screen.size();
    let mut lines = Vec::with_capacity(rows as usize);

    for r in 0..rows {
        let mut spans = Vec::new();
        for c in 0..cols {
            if let Some(cell) = screen.cell(r, c) {
                let mut style = Style::default();
                style = style
                    .fg(map_vt_color(cell.fgcolor()))
                    .bg(map_vt_color(cell.bgcolor()));
                if cell.bold() {
                    style = style.add_modifier(Modifier::BOLD);
                }
                let text = cell.contents();
                spans.push(Span::styled(
                    if text.is_empty() {
                        " ".to_string()
                    } else {
                        text.to_string()
                    },
                    style,
                ));
            }
        }
        lines.push(Line::from(spans));
    }

    let title = if let Some(s) = app.selected_session() {
        format!("PTY Output · {} ({})", s.branch_name, s.status)
    } else {
        "PTY Output".to_string()
    };
    let panel = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(panel, area);
}

fn render_diff_panel(frame: &mut Frame<'_>, app: &TuiApp, area: Rect) {
    let mut lines = Vec::new();
    if let Some(diff) = &app.current_diff {
        lines.push(Line::from(Span::styled(
            format!(
                "Base: {} | Berkas berubah: {}",
                diff.base_branch,
                diff.files.len()
            ),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for f in &diff.files {
            lines.push(Line::from(format!(
                "  {:?}: {} (+{} -{})",
                f.status, f.path, f.additions, f.deletions
            )));
        }
        lines.push(Line::from(""));
        for raw in diff.patch.lines().take(200) {
            let style = if raw.starts_with('+') && !raw.starts_with("+++") {
                Style::default().fg(Color::Green)
            } else if raw.starts_with('-') && !raw.starts_with("---") {
                Style::default().fg(Color::Red)
            } else if raw.starts_with("@@") {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(raw.to_string(), style)));
        }
    } else {
        lines.push(Line::from("Tidak ada diff dimuat."));
    }

    let p = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Diff (Esc/d untuk kembali)"),
    );
    frame.render_widget(p, area);
}

fn render_pipelines_list(frame: &mut Frame<'_>, app: &TuiApp, area: Rect) {
    let items: Vec<ListItem<'_>> = app
        .pipelines
        .iter()
        .map(|p| {
            let color = match p.status {
                PipelineStatus::Running | PipelineStatus::Pending => Color::Green,
                PipelineStatus::Succeeded => Color::Cyan,
                PipelineStatus::Failed | PipelineStatus::Tripped => Color::Red,
                PipelineStatus::Cancelled => Color::Yellow,
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("[{}] ", p.status.as_str()),
                    Style::default().fg(color),
                ),
                Span::styled(
                    p.workflow_name.as_str(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(" (${:.4})", p.total_cost_usd)),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    if !app.pipelines.is_empty() {
        state.select(Some(app.selected_pipeline));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Pipeline DAG (P/Esc: sesi, c: batal)"),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_pipeline_arena_panel(frame: &mut Frame<'_>, app: &TuiApp, area: Rect) {
    let mut lines = Vec::new();
    if let Some(run) = app.selected_pipeline() {
        lines.push(Line::from(Span::styled(
            format!(
                "Workflow: {} | Status: {} | Biaya: ${:.4}",
                run.workflow_name,
                run.status.as_str(),
                run.total_cost_usd
            ),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        if let Some(task) = &run.task {
            lines.push(Line::from(format!("Task: {task}")));
        }
        lines.push(Line::from(""));

        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&run.dag_state_json) {
            if let Some(steps) = val.get("steps").and_then(|s| s.as_object()) {
                for (step_id, step_val) in steps {
                    let st = step_val
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let winner = step_val.get("winner").and_then(|v| v.as_str());
                    lines.push(Line::from(Span::styled(
                        format!(
                            "Step '{step_id}' [{st}]{}",
                            winner.map_or(String::new(), |w| format!(" · Pemenang: {w}"))
                        ),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )));
                    if let Some(cands) = step_val.get("candidates").and_then(|c| c.as_array()) {
                        if !cands.is_empty() {
                            lines.push(Line::from(
                                "  Profil       | Status | Durasi   | Diff | Biaya",
                            ));
                            for cand in cands {
                                let prof =
                                    cand.get("profile").and_then(|v| v.as_str()).unwrap_or("-");
                                let passed = cand
                                    .get("passed")
                                    .and_then(serde_json::Value::as_bool)
                                    .unwrap_or(false);
                                let dur = cand
                                    .get("duration_ms")
                                    .and_then(serde_json::Value::as_u64)
                                    .unwrap_or(0);
                                let diff_lines = cand
                                    .get("diff_lines")
                                    .and_then(serde_json::Value::as_u64)
                                    .unwrap_or(0);
                                let cost = cand
                                    .get("cost_usd")
                                    .and_then(serde_json::Value::as_f64)
                                    .unwrap_or(0.0);
                                let star = if winner == Some(prof) { "*" } else { " " };
                                lines.push(Line::from(format!(
                                    " {star}{prof:<12} | {:<6} | {:>6}ms | {:>4} | ${cost:.4}",
                                    if passed { "LOLOS" } else { "GAGAL" },
                                    dur,
                                    diff_lines
                                )));
                            }
                        }
                    }
                }
            }
        }
    } else {
        lines.push(Line::from(
            "Belum ada pipeline. Jalankan `latch pipeline run` untuk memulai.",
        ));
    }

    let p = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Detail DAG & Fan-Out Arena"),
    );
    frame.render_widget(p, area);
}

fn render_confirm_modal(frame: &mut Frame<'_>, action: ConfirmAction, session_id: &str) {
    let area = centered_rect(54, 28, frame.area());
    frame.render_widget(Clear, area);
    let label = match action {
        ConfirmAction::Kill => "Hentikan paksa (kill)",
        ConfirmAction::Merge => "Squash merge ke branch default",
        ConfirmAction::Discard => "Buang sesi + worktree",
    };
    let text = vec![
        Line::from(format!("{label} untuk sesi {}?", short_id(session_id))),
        Line::from(""),
        Line::from("Tekan 'y' / Enter untuk konfirmasi, 'n' / Esc untuk batal."),
    ];
    let popup = Paragraph::new(text)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Konfirmasi"));
    frame.render_widget(popup, area);
}

fn render_help_modal(frame: &mut Frame<'_>) {
    let area = centered_rect(60, 50, frame.area());
    frame.render_widget(Clear, area);
    let lines = vec![
        Line::from("Pintasan Keyboard Latch TUI:"),
        Line::from("  ↑/↓, j/k    : Pilih sesi"),
        Line::from("  Enter / a   : Attach interaktif ke PTY"),
        Line::from("  Ctrl-b d    : Detach dari mode attach"),
        Line::from("  p           : Pause / Resume (SIGSTOP/SIGCONT)"),
        Line::from("  d           : Tampilkan diff worktree"),
        Line::from("  m           : Squash merge sesi (dengan konfirmasi)"),
        Line::from("  x / X       : Kill / Discard sesi (dengan konfirmasi)"),
        Line::from("  q / Esc     : Tutup modal atau keluar"),
    ];
    let popup =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Bantuan"));
    frame.render_widget(popup, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(v[1])[1]
}

const fn map_vt_color(c: vt100::Color) -> Color {
    match c {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(i) => Color::Indexed(i),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

fn encode_key_bytes(key: KeyEvent) -> Option<Vec<u8>> {
    match key.code {
        KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let lower = c.to_ascii_lowercase();
            if lower.is_ascii_lowercase() {
                Some(vec![(lower as u8) - b'a' + 1])
            } else {
                None
            }
        }
        KeyCode::Char(c) => {
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            Some(s.as_bytes().to_vec())
        }
        KeyCode::Enter => Some(vec![b'\r']),
        KeyCode::Backspace => Some(vec![0x7f]),
        KeyCode::Tab => Some(vec![b'\t']),
        KeyCode::Esc => Some(vec![0x1b]),
        KeyCode::Up => Some(b"\x1b[A".to_vec()),
        KeyCode::Down => Some(b"\x1b[B".to_vec()),
        KeyCode::Right => Some(b"\x1b[C".to_vec()),
        KeyCode::Left => Some(b"\x1b[D".to_vec()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use latch_core::id::{new_id, now_ms};
    use latch_core::model::{
        DiffFileStat, DiffFileStatus, ProxyMode, Session, SessionDiff, SessionStatus,
    };
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use super::{render_ui, ConfirmAction, TuiApp, UiMode};

    fn sample_session() -> Session {
        Session {
            id: new_id(),
            project_id: new_id(),
            agent_profile: "sleeper".to_string(),
            worktree_path: "/tmp/wt/1".to_string(),
            branch_name: "latch/demo/aabbccdd".to_string(),
            status: SessionStatus::Running,
            pid: Some(1234),
            started_at: now_ms(),
            ended_at: None,
            task: Some("demo".to_string()),
            pipeline_run_id: None,
            step_id: None,
            exit_code: None,
            trip_reason: None,
            proxy_mode: ProxyMode::Bypass,
            sandbox_provider: "noop".to_string(),
        }
    }

    #[test]
    fn renders_session_list_ansi_output_diff_and_confirm_modal() {
        let mut app = TuiApp::new();
        let ses = sample_session();
        let sid = ses.id.clone();
        app.set_sessions(vec![ses]);
        app.vt.process(b"\x1b[32mHELLO-VT100\x1b[0m");

        let backend = TestBackend::new(100, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render_ui(f, &mut app)).unwrap();
        let buf_str: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(buf_str.contains("LATCH TUI"));
        assert!(buf_str.contains("HELLO-VT100"));

        app.current_diff = Some(SessionDiff {
            base_branch: "main".to_string(),
            files: vec![DiffFileStat {
                path: "src/lib.rs".to_string(),
                old_path: None,
                status: DiffFileStatus::Modified,
                additions: 2,
                deletions: 1,
            }],
            patch: "@@ -1 +1 @@\n-old\n+new\n".to_string(),
            total_patch_bytes: 24,
            truncated: false,
        });
        app.mode = UiMode::Diff {
            session_id: sid.clone(),
        };
        term.draw(|f| render_ui(f, &mut app)).unwrap();
        let diff_str: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(diff_str.contains("src/lib.rs"));

        app.mode = UiMode::Confirm {
            action: ConfirmAction::Merge,
            session_id: sid,
        };
        term.draw(|f| render_ui(f, &mut app)).unwrap();
        let modal_str: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(modal_str.contains("Konfirmasi"));

        // Uji tampilan Pipeline & Arena (T-409)
        app.set_pipelines(vec![latch_core::model::PipelineRun {
            id: new_id(),
            project_id: new_id(),
            workflow_name: "feature-arena".to_string(),
            dag_state_json: r#"{"steps":{"implement":{"status":"succeeded","winner":"cheap","candidates":[{"profile":"cheap","passed":true,"duration_ms":300,"diff_lines":15,"cost_usd":0.1}]}}}"#.to_string(),
            status: latch_core::model::PipelineStatus::Succeeded,
            created_at: now_ms(),
            updated_at: Some(now_ms()),
            task: Some("arena test".to_string()),
            total_cost_usd: 0.1,
        }]);
        app.mode = UiMode::Pipelines;
        term.draw(|f| render_ui(f, &mut app)).unwrap();
        let pipe_str: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(pipe_str.contains("feature-arena"));
        assert!(pipe_str.contains("cheap"));
    }
}
