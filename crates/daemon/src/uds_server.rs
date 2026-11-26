//! Server Unix Domain Socket (`0600` + `SO_PEERCRED`) melayani JSON-RPC 2.0 & attach PTY (T-130).

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;

use base64::Engine;
use latch_core::protocol::{
    read_json_frame, read_raw_frame, write_json_frame, write_raw_frame, AttachFrame,
    JsonRpcRequest, JsonRpcResponse, ProjectAddParams, SessionDiffParams, SessionIdParams,
    SessionListParams, SessionMergeParams, SessionResizeParams, SessionRunParams,
};
use latch_pty_pool::PtyFrame;
use nix::unistd::geteuid;
use serde::Deserialize;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Notify;
use tracing::{debug, warn};

use crate::session_manager::{ManagerError, SessionManager};

#[derive(Deserialize)]
struct SessionInputParams {
    session_id: String,
    data_base64: String,
}

#[derive(Deserialize)]
struct IdOrNameParams {
    id_or_name: String,
}

/// Menjalankan listener UDS sampai sinyal shutdown diterima.
pub async fn serve_uds(
    socket_path: &Path,
    manager: SessionManager,
    shutdown: Arc<Notify>,
) -> anyhow::Result<()> {
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
    }

    if socket_path.exists() {
        if UnixStream::connect(socket_path).await.is_ok() {
            anyhow::bail!(
                "another latch daemon is already listening on {}",
                socket_path.display()
            );
        }
        std::fs::remove_file(socket_path)?;
    }

    let listener = UnixListener::bind(socket_path)?;
    std::fs::set_permissions(socket_path, std::fs::Permissions::from_mode(0o600))?;

    loop {
        tokio::select! {
            _ = shutdown.notified() => break,
            accept_res = listener.accept() => {
                match accept_res {
                    Ok((stream, _)) => {
                        let mgr = manager.clone();
                        tokio::spawn(async move {
                            if let Err(err) = handle_connection(stream, mgr).await {
                                debug!(error = %err, "UDS connection closed with error");
                            }
                        });
                    }
                    Err(err) => {
                        warn!(error = %err, "failed accepting UDS client");
                    }
                }
            }
        }
    }

    let _ = std::fs::remove_file(socket_path);
    Ok(())
}

async fn handle_connection(mut stream: UnixStream, mgr: SessionManager) -> anyhow::Result<()> {
    verify_peer_uid(&stream)?;

    while let Some(req) = read_json_frame::<_, JsonRpcRequest>(&mut stream).await? {
        if req.method == "session.attach" {
            let params: SessionIdParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(err) => {
                    write_json_frame(
                        &mut stream,
                        &JsonRpcResponse::err(req.id, -32602, err.to_string()),
                    )
                    .await?;
                    continue;
                }
            };
            match mgr.attach_session(&params.session_id).await {
                Ok((_ses, handle, mut sub)) => {
                    write_json_frame(
                        &mut stream,
                        &JsonRpcResponse::ok(req.id, serde_json::json!({"attached": true})),
                    )
                    .await?;
                    let snap = AttachFrame::Snapshot(sub.initial_bytes);
                    write_raw_frame(&mut stream, &snap.encode()).await?;

                    let (mut rd, mut wr) = stream.into_split();
                    loop {
                        tokio::select! {
                            maybe_frame = sub.rx.recv() => {
                                match maybe_frame {
                                    Some(PtyFrame::Output(bytes)) => {
                                        if write_raw_frame(&mut wr, &AttachFrame::Output(bytes).encode()).await.is_err() {
                                            break;
                                        }
                                    }
                                    Some(PtyFrame::Dropped(bytes)) => {
                                        if write_raw_frame(&mut wr, &AttachFrame::Dropped(bytes).encode()).await.is_err() {
                                            break;
                                        }
                                    }
                                    None => break,
                                }
                            }
                            maybe_in = read_raw_frame(&mut rd) => {
                                match maybe_in {
                                    Ok(Some(raw)) => {
                                        if let Ok(frame) = AttachFrame::decode(&raw) {
                                            match frame {
                                                AttachFrame::Input(bytes) => {
                                                    let _ = handle.write_input(&bytes);
                                                }
                                                AttachFrame::Resize { cols, rows } => {
                                                    let _ = handle.resize(cols, rows);
                                                }
                                                AttachFrame::Detach => break,
                                                AttachFrame::Output(_) | AttachFrame::Snapshot(_) | AttachFrame::Dropped(_) => {}
                                            }
                                        }
                                    }
                                    Ok(None) | Err(_) => break,
                                }
                            }
                        }
                    }
                    return Ok(());
                }
                Err(err) => {
                    write_json_frame(&mut stream, &map_err_response(req.id, err)).await?;
                    continue;
                }
            }
        }

        let resp = dispatch_rpc(req, &mgr).await;
        write_json_frame(&mut stream, &resp).await?;
    }

    Ok(())
}

fn verify_peer_uid(stream: &UnixStream) -> anyhow::Result<()> {
    let cred = stream.peer_cred()?;
    let my_uid = geteuid().as_raw();
    if cred.uid() != my_uid {
        anyhow::bail!(
            "unauthorized UDS peer UID {} (expected {})",
            cred.uid(),
            my_uid
        );
    }
    Ok(())
}

async fn dispatch_rpc(req: JsonRpcRequest, mgr: &SessionManager) -> JsonRpcResponse {
    let id = req.id;
    let res: Result<serde_json::Value, ManagerError> = match req.method.as_str() {
        "daemon.health" | "daemon.metrics" => {
            Ok(serde_json::to_value(mgr.health().await).unwrap_or(serde_json::Value::Null))
        }
        "daemon.stop" => {
            mgr.request_shutdown().await;
            Ok(serde_json::json!({ "stopping": true }))
        }
        "project.add" => match serde_json::from_value::<ProjectAddParams>(req.params) {
            Ok(p) => mgr
                .add_project(p)
                .await
                .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null)),
            Err(e) => Err(ManagerError::Config(e.to_string())),
        },
        "project.list" => mgr
            .list_projects()
            .await
            .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null)),
        "project.remove" => match serde_json::from_value::<IdOrNameParams>(req.params) {
            Ok(p) => mgr
                .remove_project(&p.id_or_name)
                .await
                .map(|()| serde_json::json!({ "removed": true })),
            Err(e) => Err(ManagerError::Config(e.to_string())),
        },
        "session.run" => match serde_json::from_value::<SessionRunParams>(req.params) {
            Ok(p) => mgr
                .run_session(p)
                .await
                .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null)),
            Err(e) => Err(ManagerError::Config(e.to_string())),
        },
        "session.list" => match serde_json::from_value::<SessionListParams>(req.params) {
            Ok(p) => mgr
                .list_sessions(p)
                .await
                .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null)),
            Err(e) => Err(ManagerError::Config(e.to_string())),
        },
        "session.get" => match serde_json::from_value::<SessionIdParams>(req.params) {
            Ok(p) => mgr
                .get_session(&p.session_id)
                .await
                .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null)),
            Err(e) => Err(ManagerError::Config(e.to_string())),
        },
        "session.pause" => match serde_json::from_value::<SessionIdParams>(req.params) {
            Ok(p) => mgr
                .pause_session(&p.session_id)
                .await
                .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null)),
            Err(e) => Err(ManagerError::Config(e.to_string())),
        },
        "session.resume" => match serde_json::from_value::<SessionIdParams>(req.params) {
            Ok(p) => mgr
                .resume_session(&p.session_id)
                .await
                .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null)),
            Err(e) => Err(ManagerError::Config(e.to_string())),
        },
        "session.kill" => match serde_json::from_value::<SessionIdParams>(req.params) {
            Ok(p) => mgr
                .kill_session(&p.session_id)
                .await
                .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null)),
            Err(e) => Err(ManagerError::Config(e.to_string())),
        },
        "session.input" => match serde_json::from_value::<SessionInputParams>(req.params) {
            Ok(p) => match base64::engine::general_purpose::STANDARD.decode(&p.data_base64) {
                Ok(bytes) => mgr
                    .send_input(&p.session_id, &bytes)
                    .await
                    .map(|()| serde_json::json!({ "ok": true })),
                Err(e) => Err(ManagerError::Config(format!("invalid base64: {e}"))),
            },
            Err(e) => Err(ManagerError::Config(e.to_string())),
        },
        "session.resize" => match serde_json::from_value::<SessionResizeParams>(req.params) {
            Ok(p) => mgr
                .resize_session(&p.session_id, p.cols, p.rows)
                .await
                .map(|()| serde_json::json!({ "ok": true })),
            Err(e) => Err(ManagerError::Config(e.to_string())),
        },
        "session.diff" => match serde_json::from_value::<SessionDiffParams>(req.params) {
            Ok(p) => mgr
                .diff_session(p)
                .await
                .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null)),
            Err(e) => Err(ManagerError::Config(e.to_string())),
        },
        "session.merge_check" => match serde_json::from_value::<SessionIdParams>(req.params) {
            Ok(p) => mgr
                .merge_check_session(&p.session_id)
                .await
                .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null)),
            Err(e) => Err(ManagerError::Config(e.to_string())),
        },
        "session.merge" => match serde_json::from_value::<SessionMergeParams>(req.params) {
            Ok(p) => mgr
                .merge_session(p)
                .await
                .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null)),
            Err(e) => Err(ManagerError::Config(e.to_string())),
        },
        "session.discard" => match serde_json::from_value::<SessionIdParams>(req.params) {
            Ok(p) => mgr
                .discard_session(&p.session_id)
                .await
                .map(|()| serde_json::json!({ "discarded": true })),
            Err(e) => Err(ManagerError::Config(e.to_string())),
        },
        "session.screen" => match serde_json::from_value::<SessionIdParams>(req.params) {
            Ok(p) => mgr
                .screen_snapshot(&p.session_id)
                .await
                .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null)),
            Err(e) => Err(ManagerError::Config(e.to_string())),
        },
        other => return JsonRpcResponse::err(id, -32601, format!("unknown method '{other}'")),
    };

    match res {
        Ok(val) => JsonRpcResponse::ok(id, val),
        Err(err) => map_err_response(id, err),
    }
}

fn map_err_response(id: u64, err: ManagerError) -> JsonRpcResponse {
    let code = match &err {
        ManagerError::Store(latch_store::StoreError::NotFound { .. }) => -32004,
        ManagerError::Store(latch_store::StoreError::InvalidTransition { .. })
        | ManagerError::InvalidState { .. } => -32009,
        ManagerError::Git(latch_git_engine::GitError::MergeConflict(_)) => -32010,
        ManagerError::Config(_) => -32602,
        _ => -32000,
    };
    JsonRpcResponse::err(id, code, err.to_string())
}
