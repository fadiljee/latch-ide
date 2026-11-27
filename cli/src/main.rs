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
    read_raw_frame, write_raw_frame, AttachFrame, PipelineArenaSelectParams, PipelineListParams,
    PipelineRunParams, ProjectAddParams, SessionDiffParams, SessionListParams, SessionMergeParams,
    SessionRunParams, UdsClient,
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
    /// Jalankan dan kelola pipeline DAG `workflow.toml`.
    Pipeline {
        #[command(subcommand)]
        action: PipelineCmd,
    },
    /// Validasi berkas `config.toml` dan `agents.toml`.
    Config {
        #[command(subcommand)]
        action: ConfigCmd,
    },
    /// Kelola bearer token autentikasi Web/WS (`0600`).
    Token {
        #[command(subcommand)]
        action: TokenCmd,
    },
    /// Buka terowongan SSH lokal ke daemon Latch di host remote (FR-03, tanpa listener publik).
    Remote {
        /// Target SSH (mis. `user@devbox` atau `ssh://user@devbox`).
        target: String,
        /// Port lokal loopback untuk meneruskan HTTP/WS daemon (bawaan 7878).
        #[arg(long, default_value_t = 7878)]
        local_port: u16,
        /// Port loopback daemon di host remote (bawaan 7878).
        #[arg(long, default_value_t = 7878)]
        remote_port: u16,
    },
    /// Bersihkan telemetri/audit kedaluwarsa serta tmp & log PTY lama (`T-411`).
    Gc {
        /// Simulasikan tanpa menghapus data.
        #[arg(long)]
        dry_run: bool,
    },
    /// Buka antarmuka terminal interaktif (TUI).
    Tui,
}

#[derive(Subcommand)]
enum TokenCmd {
    /// Tampilkan bearer token aktif (membuat baru bila belum ada).
    Show,
    /// Buat ulang bearer token 256-bit baru (`0600`).
    Rotate,
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
    /// Lanjutkan sesi (`SIGCONT` / bangun dari hibernasi).
    Resume { id: String },
    /// Hibernasi sesi aktif (simpan snapshot layar & lepas PTY untuk hemat RAM).
    Hibernate { id: String },
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

#[derive(Subcommand)]
enum PipelineCmd {
    /// Jalankan pipeline DAG dari berkas `workflow.toml`.
    Run {
        #[arg(long)]
        project: String,
        #[arg(long)]
        workflow: String,
        #[arg(long)]
        task: String,
    },
    /// Tampilkan daftar eksekusi pipeline.
    Ls {
        #[arg(long)]
        project: Option<String>,
    },
    /// Tampilkan detail & status langkah satu pipeline.
    Show { id: String },
    /// Batalkan pipeline yang sedang berjalan.
    Cancel { id: String },
    /// Pilih pemenang arena pada langkah `fanout`.
    Select {
        id: String,
        #[arg(long)]
        step: String,
        #[arg(long)]
        winner: String,
    },
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
        Commands::Token {
            action: TokenCmd::Show,
        } => {
            let tok = latch_daemon::http_server::ensure_or_load_token(&paths.token_file())?;
            println!("{tok}");
            Ok(())
        }
        Commands::Token {
            action: TokenCmd::Rotate,
        } => {
            let tok = latch_daemon::http_server::rotate_token(&paths.token_file())?;
            println!("{tok}");
            Ok(())
        }
        Commands::Remote {
            target,
            local_port,
            remote_port,
        } => run_remote_tunnel(&target, local_port, remote_port),
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

fn run_remote_tunnel(target: &str, local_port: u16, remote_port: u16) -> anyhow::Result<()> {
    let host = target.strip_prefix("ssh://").unwrap_or(target);
    let forward = format!("127.0.0.1:{local_port}:127.0.0.1:{remote_port}");
    println!("Membuka SSH port forward ke {host} (http://127.0.0.1:{local_port}) ...");
    println!("Tekan Ctrl-C untuk menutup terowongan.");
    let status = Command::new("ssh")
        .args(["-N", "-L", &forward, host])
        .status()
        .context("failed spawning ssh client for remote tunnel")?;
    if !status.success() {
        anyhow::bail!("ssh tunnel exited with status {status}");
    }
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
            Commands::Pipeline { action } => handle_pipeline_cmd(&client, action).await?,
            Commands::Gc { dry_run } => {
                let r = client.gc(dry_run).await?;
                println!(
                    "GC ({}): telemetry={} audit={} tmp_dirs={} logs={} bytes_freed={}",
                    if r.dry_run { "dry-run" } else { "applied" },
                    r.telemetry_deleted,
                    r.audit_deleted,
                    r.tmp_dirs_removed,
                    r.log_files_removed,
                    r.bytes_freed
                );
            }
            Commands::Tui => {
                latch_tui::run_tui(paths.socket_path).await?;
            }
            Commands::Daemon {
                action: DaemonCmd::Start { .. },
            }
            | Commands::Config { .. }
            | Commands::Token { .. }
            | Commands::Remote { .. } => {}
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
        SessionCmd::Hibernate { id } => {
            let s = client.session_hibernate(&id).await?;
            println!(
                "Sesi {} dihibernasi (status={}).",
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

async fn handle_pipeline_cmd(client: &UdsClient, action: PipelineCmd) -> anyhow::Result<()> {
    match action {
        PipelineCmd::Run {
            project,
            workflow,
            task,
        } => {
            let abs = std::fs::canonicalize(&workflow)
                .unwrap_or_else(|_| Path::new(&workflow).to_path_buf());
            let run = client
                .pipeline_run(&PipelineRunParams {
                    project,
                    workflow_toml: None,
                    workflow_path: Some(abs.to_string_lossy().to_string()),
                    task,
                })
                .await?;
            println!(
                "Pipeline berjalan: id={} short={} workflow={} status={}",
                run.id,
                short_id(&run.id),
                run.workflow_name,
                run.status.as_str()
            );
        }
        PipelineCmd::Ls { project } => {
            let list = client
                .pipeline_list(&PipelineListParams { project })
                .await?;
            for r in list {
                println!(
                    "{}\t{}\t{}\t${:.4}\t{}",
                    short_id(&r.id),
                    r.status.as_str(),
                    r.workflow_name,
                    r.total_cost_usd,
                    r.id
                );
            }
        }
        PipelineCmd::Show { id } => {
            let r = client.pipeline_get(&id).await?;
            println!("ID       : {}", r.id);
            println!("Workflow : {}", r.workflow_name);
            println!("Status   : {}", r.status.as_str());
            println!("Cost     : ${:.4}", r.total_cost_usd);
            println!("Created  : {}", ms_to_rfc3339(r.created_at));
            if let Some(updated) = r.updated_at {
                println!("Updated  : {}", ms_to_rfc3339(updated));
            }
            if let Some(task) = r.task {
                println!("Task     : {task}");
            }
            println!("State    : {}", r.dag_state_json);
        }
        PipelineCmd::Cancel { id } => {
            let r = client.pipeline_cancel(&id).await?;
            println!(
                "Pipeline {} dibatalkan (status={}).",
                short_id(&r.id),
                r.status.as_str()
            );
        }
        PipelineCmd::Select { id, step, winner } => {
            let r = client
                .pipeline_arena_select(&PipelineArenaSelectParams {
                    pipeline_id: id,
                    step_id: step,
                    session_id: winner,
                })
                .await?;
            println!(
                "Pemenang arena pipeline {} diperbarui (status={}).",
                short_id(&r.id),
                r.status.as_str()
            );
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
