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
use latch_core::model::{format_scopes_csv, parse_scopes_csv, MergeStrategy};
use latch_core::protocol::{
    read_raw_frame, write_raw_frame, AttachFrame, PipelineArenaSelectParams, PipelineListParams,
    PipelineRunParams, ProjectAddParams, SessionDiffParams, SessionListParams, SessionMergeParams,
    SessionRunParams, TokenCreateParams, UdsClient,
};

#[derive(Parser)]
#[command(
    name = "latch",
    version,
    about = "Latch — runtime & orkestrator AI coding agent dalam satu static binary"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
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
    /// Buka Agent IDE Workbench di browser dengan token fragment aman (IT-103, FR-I102).
    Ide {
        /// Jalur repositori Git target (bawaan: direktori saat ini bila repo Git).
        #[arg(long)]
        path: Option<String>,
        /// Jangan buka browser secara otomatis (hanya menyalakan daemon & mencetak URL).
        #[arg(long)]
        no_browser: bool,
        /// Tampilkan bearer token di terminal (nonaktif bawaan agar tidak bocor ke log terminal).
        #[arg(long)]
        print_token: bool,
    },
    /// Buka antarmuka Web Workbench di browser (auto-start daemon).
    #[command(alias = "open")]
    Web,
    /// Buka antarmuka terminal interaktif (TUI).
    Tui,
}

#[derive(Subcommand)]
enum TokenCmd {
    /// Tampilkan bearer token pemilik lokal aktif (membuat baru bila belum ada).
    Show,
    /// Buat ulang bearer token pemilik lokal 256-bit baru (`0600`).
    Rotate,
    /// Buat bearer token ber-scope baru (disimpan sebagai hash SHA-256; token mentah ditampilkan sekali).
    Create {
        /// Daftar scope dipisahkan koma: read,control,write,shell-sandbox,shell-host,admin.
        #[arg(long)]
        scopes: String,
        /// Label ramah pengguna (mis. `hp`, `readonly`).
        #[arg(long, default_value = "custom")]
        label: String,
        /// Masa berlaku opsional (mis. `30m`, `12h`, `7d`).
        #[arg(long)]
        expires: Option<String>,
    },
    /// Tampilkan daftar token ber-scope terdaftar (tanpa nilai rahasia).
    Ls,
    /// Cabut (revoke) token ber-scope berdasarkan ID atau prefiks ID.
    Revoke {
        /// ID penuh atau prefiks ID token.
        id: String,
    },
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
    let command = cli.command.unwrap_or(Commands::Tui);
    match command {
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
        Commands::Ide {
            path,
            no_browser,
            print_token,
        } => run_open_workbench(&paths, path.as_deref(), !no_browser, print_token),
        Commands::Web => run_open_workbench(&paths, None, true, false),
        other => run_async_client(paths, other),
    }
}

fn parse_duration_secs(raw: &str) -> anyhow::Result<u64> {
    let trimmed = raw.trim();
    if trimmed.len() < 2 {
        anyhow::bail!("invalid duration '{raw}' (use e.g. 30m, 12h, 7d)");
    }
    let (num_part, unit) = trimmed.split_at(trimmed.len() - 1);
    let val: u64 = num_part
        .parse()
        .with_context(|| format!("invalid duration number in '{raw}'"))?;
    let mult = match unit {
        "s" => 1,
        "m" => 60,
        "h" => 3_600,
        "d" => 86_400,
        _ => anyhow::bail!("invalid duration unit '{unit}' in '{raw}' (expected s, m, h, or d)"),
    };
    val.checked_mul(mult)
        .filter(|&v| v > 0)
        .ok_or_else(|| anyhow::anyhow!("duration '{raw}' out of range"))
}

fn ensure_daemon_running(paths: &RuntimePaths) -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let client = UdsClient::new(&paths.socket_path);
    if rt.block_on(client.health()).is_ok() {
        return Ok(());
    }
    spawn_background_daemon(paths)
}

fn build_workbench_fragment_url(http_bind: &str, token: &str) -> String {
    format!("http://{http_bind}/#token={token}")
}

fn open_browser_url(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = Command::new("open")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    #[cfg(target_os = "windows")]
    let _ = Command::new("cmd")
        .args(["/C", "start", "", url])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = Command::new("xdg-open")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn run_open_workbench(
    paths: &RuntimePaths,
    repo_path: Option<&str>,
    launch_browser: bool,
    print_token: bool,
) -> anyhow::Result<()> {
    ensure_daemon_running(paths)?;

    // Auto-daftarkan repo target / CWD bila merupakan repositori Git.
    let target_dir = match repo_path {
        Some(p) => Some(Path::new(p).to_path_buf()),
        None => std::env::current_dir().ok(),
    };
    if let Some(dir) = target_dir {
        if let Ok(abs) = std::fs::canonicalize(&dir) {
            if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                let client = UdsClient::new(&paths.socket_path);
                let _ = rt.block_on(client.project_add(&ProjectAddParams {
                    repo_path: abs.to_string_lossy().to_string(),
                    name: None,
                    default_branch: None,
                }));
            }
        }
    }

    let cfg = load_latch_config(&paths.config_file()).unwrap_or_default();
    let tok = latch_daemon::http_server::ensure_or_load_token(&paths.token_file())?;
    let base_url = format!("http://{}", cfg.daemon.http_bind);
    let auth_url = build_workbench_fragment_url(&cfg.daemon.http_bind, &tok);
    println!("Latch Agent IDE : {base_url}");
    if print_token {
        println!("Bearer Token    : {tok}");
    } else {
        println!("Autentikasi     : diteruskan via URL fragment aman (gunakan `latch token show` bila perlu manual).");
    }
    if launch_browser {
        open_browser_url(&auth_url);
    }
    Ok(())
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
    if !matches!(cmd, Commands::Daemon { .. }) {
        ensure_daemon_running(&paths)?;
    }
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
                // Auto-register repositori Git di direktori saat ini bila belum terdaftar (zero-setup).
                if let Ok(cwd) = std::env::current_dir() {
                    if let Ok(abs) = std::fs::canonicalize(&cwd) {
                        let _ = client
                            .project_add(&ProjectAddParams {
                                repo_path: abs.to_string_lossy().to_string(),
                                name: None,
                                default_branch: None,
                            })
                            .await;
                    }
                }
                latch_tui::run_tui(paths.socket_path).await?;
            }
            Commands::Token { action } => match action {
                TokenCmd::Create {
                    scopes,
                    label,
                    expires,
                } => {
                    let parsed_scopes = parse_scopes_csv(&scopes)
                        .map_err(|e| anyhow::anyhow!("invalid --scopes: {e}"))?;
                    let expires_in_secs = match expires {
                        Some(raw) => Some(parse_duration_secs(&raw)?),
                        None => None,
                    };
                    let created = client
                        .token_create(&TokenCreateParams {
                            label,
                            scopes: parsed_scopes,
                            expires_in_secs,
                        })
                        .await?;
                    println!(
                        "id={} label={} scopes={}",
                        created.metadata.id,
                        created.metadata.label,
                        format_scopes_csv(&created.metadata.scopes)
                    );
                    if let Some(exp) = created.metadata.expires_at {
                        println!("expires={}", ms_to_rfc3339(exp));
                    }
                    println!("token={}", created.token);
                    println!("Simpan token ini sekarang; nilai mentah tidak disimpan di daemon.");
                }
                TokenCmd::Ls => {
                    let tokens = client.token_list().await?;
                    if tokens.is_empty() {
                        println!("Belum ada token ber-scope tambahan.");
                    } else {
                        for tok in tokens {
                            let state = if tok.revoked_at.is_some() {
                                "revoked"
                            } else {
                                "active"
                            };
                            let exp = tok
                                .expires_at
                                .map(ms_to_rfc3339)
                                .unwrap_or_else(|| "never".to_string());
                            println!(
                                "{}\t{}\t{}\t{}\texpires={}",
                                short_id(&tok.id),
                                state,
                                tok.label,
                                format_scopes_csv(&tok.scopes),
                                exp
                            );
                        }
                    }
                }
                TokenCmd::Revoke { id } => {
                    let revoked = client.token_revoke(&id).await?;
                    println!(
                        "Token {} (label={}) berhasil dicabut.",
                        short_id(&revoked.id),
                        revoked.label
                    );
                }
                TokenCmd::Show | TokenCmd::Rotate => {}
            },
            Commands::Daemon {
                action: DaemonCmd::Start { .. },
            }
            | Commands::Config { .. }
            | Commands::Remote { .. }
            | Commands::Ide { .. }
            | Commands::Web => {}
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

#[cfg(test)]
mod tests {
    use super::{build_workbench_fragment_url, parse_duration_secs};

    #[test]
    fn builds_fragment_url_without_query_token() {
        let url = build_workbench_fragment_url("127.0.0.1:7878", "secret-tok-abc");
        assert_eq!(url, "http://127.0.0.1:7878/#token=secret-tok-abc");
        assert!(!url.contains("?token="));
    }

    #[test]
    fn parses_human_duration_strings() {
        assert_eq!(parse_duration_secs("30s").unwrap(), 30);
        assert_eq!(parse_duration_secs("15m").unwrap(), 900);
        assert_eq!(parse_duration_secs("12h").unwrap(), 43_200);
        assert_eq!(parse_duration_secs("7d").unwrap(), 604_800);
        assert!(parse_duration_secs("0d").is_err());
        assert!(parse_duration_secs("bad").is_err());
    }
}
