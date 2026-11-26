#![forbid(unsafe_code)]
#![allow(clippy::print_stdout)]
//! Entrypoint tunggal Latch (`latch` & alias `latchd` lewat `argv[0]`).

use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::Context;
use clap::{Args, Parser, Subcommand};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use latch_core::config::{load_agents_config, load_latch_config, RuntimePaths};
use latch_core::id::{ms_to_rfc3339, short_id};
use latch_core::model::MergeStrategy;
use latch_core::protocol::{
    read_raw_frame, write_raw_frame, AttachFrame, ProjectAddParams, SessionDiffParams,
    SessionListParams, SessionMergeParams, SessionRunParams, UdsClient,
};

#[derive(Parser)]
#[command(
    name = "latch",
    version,
    about = "Latch — runtime & orkestrator AI coding agent dalam satu static binary"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Kelola proses daemon latar belakang.
    Daemon {
        #[command(subcommand)]
        action: DaemonCmd,
    },
    /// Kelola proyek Git terdaftar.
    Project {
        #[command(subcommand)]
        action: ProjectCmd,
    },
    /// Jalankan satu sesi agen pada worktree baru.
    Run(RunArgs),
    /// Kelola dan tinjau sesi agen.
    Session {
        #[command(subcommand)]
        action: SessionCmd,
    },
    /// Validasi berkas `config.toml` dan `agents.toml`.
    Config {
        #[command(subcommand)]
        action: ConfigCmd,
    },
    /// Buka antarmuka terminal interaktif (TUI).
    Tui,
}

#[derive(Subcommand)]
enum DaemonCmd {
    /// Mulai daemon Latch.
    Start {
        /// Jalankan di foreground (cocok untuk systemd / supervisor).
        #[arg(long)]
        foreground: bool,
    },
    /// Hentikan daemon yang sedang berjalan.
    Stop,
    /// Periksa status dan kesehatan daemon.
    Status,
}

#[derive(Subcommand)]
enum ProjectCmd {
    /// Daftarkan repositori Git ke daemon.
    Add {
        repo_path: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        default_branch: Option<String>,
    },
    /// Tampilkan daftar proyek terdaftar.
    Ls,
    /// Hapus proyek terdaftar (gagal bila masih ada sesi aktif).
    Rm { id_or_name: String },
}

#[derive(Args)]
struct RunArgs {
    #[arg(long)]
    project: String,
    #[arg(long)]
    profile: String,
    #[arg(long)]
    task: String,
    #[arg(long)]
    base_ref: Option<String>,
}

#[derive(Subcommand)]
enum SessionCmd {
    /// Tampilkan daftar sesi.
    Ls {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    /// Tampilkan detail satu sesi.
    Show { id: String },
    /// Attach interaktif ke PTY sesi (Ctrl-b d untuk detach).
    Attach { id: String },
    /// Jeda sesi (`SIGSTOP`).
    Pause { id: String },
    /// Lanjutkan sesi (`SIGCONT`).
    Resume { id: String },
    /// Hentikan paksa sesi.
    Kill { id: String },
    /// Tampilkan diff worktree sesi.
    Diff {
        id: String,
        #[arg(long)]
        file: Option<String>,
    },
    /// Simulasikan merge (`git merge-tree`) untuk memeriksa konflik.
    MergeCheck { id: String },
    /// Gabungkan hasil sesi ke branch default.
    Merge {
        id: String,
        #[arg(long, conflicts_with = "merge")]
        squash: bool,
        #[arg(long, conflicts_with = "squash")]
        merge: bool,
        #[arg(long)]
        message: Option<String>,
    },
    /// Buang sesi beserta worktree dan branch-nya.
    Discard { id: String },
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// Validasi sintaks dan aturan konfigurasi.
    Check,
}

fn main() -> anyhow::Result<()> {
    let paths = RuntimePaths::resolve();

    // Alias argv[0] = "latchd" langsung menjalankan mode daemon foreground (FR-01)
    if let Some(arg0) = std::env::args_os().next() {
        let exe_name = Path::new(&arg0)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("latch");
        if exe_name == "latchd" && std::env::args_os().nth(1).is_none() {
            return latch_daemon::run_foreground(paths);
        }
    }

    let cli = Cli::parse();
    match cli.command {
        Commands::Daemon {
            action: DaemonCmd::Start { foreground: true },
        } => latch_daemon::run_foreground(paths),
        Commands::Daemon {
            action: DaemonCmd::Start { foreground: false },
        } => spawn_background_daemon(&paths),
        Commands::Config {
            action: ConfigCmd::Check,
        } => run_config_check(&paths),
        other => run_async_client(paths, other),
    }
}

fn spawn_background_daemon(paths: &RuntimePaths) -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let client = UdsClient::new(&paths.socket_path);
    if rt.block_on(client.health()).is_ok() {
        println!(
            "Daemon Latch sudah berjalan di {}",
            paths.socket_path.display()
        );
        return Ok(());
    }

    let exe = std::env::current_exe().context("cannot determine current executable path")?;
    std::fs::create_dir_all(paths.logs_dir())?;
    let child = Command::new(exe)
        .args(["daemon", "start", "--foreground"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed spawning background latch daemon")?;

    for _ in 0..50 {
        if rt.block_on(client.health()).is_ok() {
            println!(
                "Daemon Latch berjalan (PID {}) di {}",
                child.id(),
                paths.socket_path.display()
            );
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    anyhow::bail!(
        "daemon started (PID {}) but socket {} did not become ready in time",
        child.id(),
        paths.socket_path.display()
    );
}

fn run_config_check(paths: &RuntimePaths) -> anyhow::Result<()> {
    let cfg_path = paths.config_file();
    let agents_path = paths.agents_file();
    let cfg = load_latch_config(&cfg_path)?;
    let agents = load_agents_config(&agents_path)?;
    println!(
        "OK: config={} ({}), agents={} ({} profil)",
        cfg_path.display(),
        if cfg_path.exists() { "file" } else { "default" },
        agents_path.display(),
        agents.profiles.len(),
    );
    println!(
        "  worker_threads={}, max_sessions={}, pty_batch_ms={}ms",
        cfg.daemon.worker_threads, cfg.limits.max_sessions, cfg.limits.pty_batch_ms
    );
    Ok(())
}

fn run_async_client(paths: RuntimePaths, cmd: Commands) -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()?;

    rt.block_on(async move {
        let client = UdsClient::new(&paths.socket_path);
        match cmd {
            Commands::Daemon {
                action: DaemonCmd::Stop,
            } => {
                let _: serde_json::Value =
                    client.call("daemon.stop", &serde_json::json!({})).await?;
                println!("Permintaan stop dikirim ke daemon.");
            }
            Commands::Daemon {
                action: DaemonCmd::Status,
            } => {
                let h = client.health().await?;
                println!(
                    "status={} version={} pid={} sandbox={} active_sessions={}",
                    h.status, h.version, h.pid, h.sandbox, h.active_sessions
                );
            }
            Commands::Project { action } => match action {
                ProjectCmd::Add {
                    repo_path,
                    name,
                    default_branch,
                } => {
                    let abs = std::fs::canonicalize(&repo_path)
                        .unwrap_or_else(|_| Path::new(&repo_path).to_path_buf());
                    let p = client
                        .project_add(&ProjectAddParams {
                            repo_path: abs.to_string_lossy().to_string(),
                            name,
                            default_branch,
                        })
                        .await?;
                    println!(
                        "Proyek ditambahkan: {} (id={}, default_branch={})",
                        p.name, p.id, p.default_branch
                    );
                }
                ProjectCmd::Ls => {
                    let list = client.project_list().await?;
                    for p in list {
                        println!(
                            "{}\t{}\t{}\t{}",
                            p.name, p.default_branch, p.repo_path, p.id
                        );
                    }
                }
                ProjectCmd::Rm { id_or_name } => {
                    let _: serde_json::Value = client
                        .call(
                            "project.remove",
                            &serde_json::json!({ "id_or_name": id_or_name }),
                        )
                        .await?;
                    println!("Proyek dihapus.");
                }
            },
            Commands::Run(args) => {
                let s = client
                    .session_run(&SessionRunParams {
                        project: args.project,
                        profile: args.profile,
                        task: args.task,
                        base_ref: args.base_ref,
                    })
                    .await?;
                println!(
                    "Sesi berjalan: id={} short={} branch={} worktree={}",
                    s.id,
                    short_id(&s.id),
                    s.branch_name,
                    s.worktree_path
                );
            }
            Commands::Session { action } => handle_session_cmd(&client, action).await?,
            Commands::Tui => {
                latch_tui::run_tui(paths.socket_path).await?;
            }
            Commands::Daemon {
                action: DaemonCmd::Start { .. },
            }
            | Commands::Config { .. } => {}
        }
        Ok(())
    })
}

async fn handle_session_cmd(client: &UdsClient, action: SessionCmd) -> anyhow::Result<()> {
    match action {
        SessionCmd::Ls { project, status } => {
            let sessions = client
                .session_list(&SessionListParams { project, status })
                .await?;
            for s in sessions {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    short_id(&s.id),
                    s.status,
                    s.agent_profile,
                    s.branch_name,
                    s.id
                );
            }
        }
        SessionCmd::Show { id } => {
            let s = client.session_get(&id).await?;
            println!("ID       : {}", s.id);
            println!("Status   : {}", s.status);
            println!("Profile  : {}", s.agent_profile);
            println!("Branch   : {}", s.branch_name);
            println!("Worktree : {}", s.worktree_path);
            println!("Started  : {}", ms_to_rfc3339(s.started_at));
            if let Some(ended) = s.ended_at {
                println!("Ended    : {}", ms_to_rfc3339(ended));
            }
            if let Some(code) = s.exit_code {
                println!("ExitCode : {code}");
            }
        }
        SessionCmd::Attach { id } => attach_cli_pty(client, &id).await?,
        SessionCmd::Pause { id } => {
            let s = client.session_pause(&id).await?;
            println!("Sesi {} dijeda (status={}).", short_id(&s.id), s.status);
        }
        SessionCmd::Resume { id } => {
            let s = client.session_resume(&id).await?;
            println!(
                "Sesi {} dilanjutkan (status={}).",
                short_id(&s.id),
                s.status
            );
        }
        SessionCmd::Kill { id } => {
            let s = client.session_kill(&id).await?;
            println!("Sesi {} dihentikan (status={}).", short_id(&s.id), s.status);
        }
        SessionCmd::Diff { id, file } => {
            let diff = client
                .session_diff(&SessionDiffParams {
                    session_id: id,
                    file,
                    max_bytes: None,
                })
                .await?;
            println!("Base: {} ({} berkas)", diff.base_branch, diff.files.len());
            for f in &diff.files {
                println!(
                    "  {:?} {} (+{} -{})",
                    f.status, f.path, f.additions, f.deletions
                );
            }
            println!("\n{}", diff.patch);
            if diff.truncated {
                println!("[diff dipotong; total {} byte]", diff.total_patch_bytes);
            }
        }
        SessionCmd::MergeCheck { id } => {
            let chk = client.session_merge_check(&id).await?;
            if chk.clean {
                println!("Bersih: siap digabungkan.");
            } else {
                println!("Konflik terdeteksi pada: {}", chk.conflicts.join(", "));
            }
        }
        SessionCmd::Merge {
            id,
            squash: _,
            merge,
            message,
        } => {
            let strategy = if merge {
                MergeStrategy::Merge
            } else {
                MergeStrategy::Squash
            };
            let s = client
                .session_merge(&SessionMergeParams {
                    session_id: id,
                    strategy,
                    message,
                })
                .await?;
            println!(
                "Sesi {} selesai digabungkan dan worktree dibersihkan.",
                short_id(&s.id)
            );
        }
        SessionCmd::Discard { id } => {
            client.session_discard(&id).await?;
            println!("Sesi dibuang dan worktree dibersihkan.");
        }
    }
    Ok(())
}

async fn attach_cli_pty(client: &UdsClient, session_id: &str) -> anyhow::Result<()> {
    let stream = client.attach_stream(session_id).await?;
    let (mut rd, mut wr) = stream.into_split();

    enable_raw_mode()?;
    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    let _ = write_raw_frame(&mut wr, &AttachFrame::Resize { cols, rows }.encode()).await;

    let (in_tx, mut in_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(32);
    let (resize_tx, mut resize_rx) = tokio::sync::mpsc::channel::<(u16, u16)>(8);
    let (detach_tx, mut detach_rx) = tokio::sync::mpsc::channel::<()>(1);

    std::thread::spawn(move || {
        let mut armed = false;
        while let Ok(has_ev) = event::poll(Duration::from_millis(100)) {
            if !has_ev {
                continue;
            }
            let Ok(ev) = event::read() else {
                break;
            };
            match ev {
                Event::Resize(c, r) => {
                    let _ = resize_tx.blocking_send((c, r));
                }
                Event::Key(k) => {
                    if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('b') {
                        armed = true;
                        continue;
                    }
                    if armed {
                        armed = false;
                        if k.code == KeyCode::Char('d') {
                            let _ = detach_tx.blocking_send(());
                            break;
                        }
                    }
                    if let KeyCode::Char(c) = k.code {
                        if k.modifiers.contains(KeyModifiers::CONTROL) && c.is_ascii_alphabetic() {
                            let ctrl = (c.to_ascii_lowercase() as u8) - b'a' + 1;
                            if in_tx.blocking_send(vec![ctrl]).is_err() {
                                break;
                            }
                            continue;
                        }
                        let mut b = [0u8; 4];
                        let s = c.encode_utf8(&mut b);
                        if in_tx.blocking_send(s.as_bytes().to_vec()).is_err() {
                            break;
                        }
                    } else if k.code == KeyCode::Enter {
                        let _ = in_tx.blocking_send(vec![b'\r']);
                    } else if k.code == KeyCode::Backspace {
                        let _ = in_tx.blocking_send(vec![0x7f]);
                    }
                }
                _ => {}
            }
        }
        let mut stdin = io::stdin();
        let mut dummy = [0u8; 0];
        let _ = stdin.read(&mut dummy);
    });

    let mut stdout = io::stdout();
    loop {
        tokio::select! {
            _ = detach_rx.recv() => {
                let _ = write_raw_frame(&mut wr, &AttachFrame::Detach.encode()).await;
                break;
            }
            Some(bytes) = in_rx.recv() => {
                if write_raw_frame(&mut wr, &AttachFrame::Input(bytes).encode()).await.is_err() {
                    break;
                }
            }
            Some((cols, rows)) = resize_rx.recv() => {
                let _ = write_raw_frame(&mut wr, &AttachFrame::Resize { cols, rows }.encode()).await;
            }
            frame_res = read_raw_frame(&mut rd) => {
                match frame_res {
                    Ok(Some(raw)) => {
                        if let Ok(frame) = AttachFrame::decode(&raw) {
                            match frame {
                                AttachFrame::Snapshot(b) | AttachFrame::Output(b) => {
                                    let _ = stdout.write_all(&b);
                                    let _ = stdout.flush();
                                }
                                AttachFrame::Dropped(_) => {}
                                AttachFrame::Detach => break,
                                AttachFrame::Input(_) | AttachFrame::Resize { .. } => {}
                            }
                        }
                    }
                    Ok(None) | Err(_) => break,
                }
            }
        }
    }

    disable_raw_mode()?;
    println!("\r\n[detached]");
    Ok(())
}
