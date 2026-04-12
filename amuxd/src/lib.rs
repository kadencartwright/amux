pub mod shell;
pub mod store;
pub mod terminal;

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path as AxumPath, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, SecondsFormat, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use store::{
    ControlStore, ManagedWorktree, NewManagedWorktree, NewStoredSession, NewWorkspace,
    SessionKind, SourceRef, SourceRefKind, StoredSession, Workspace, WorkspaceKind,
    basename_for_path,
};
use terminal::{
    TerminalCore, TerminalCursor, TerminalInputEvent, TerminalInputRequest, TerminalInputResponse,
    TerminalLine, TerminalSnapshot, TerminalStreamFrame, TerminalSurfaceState,
};
use tokio::sync::{broadcast, watch};
use tokio::time::{self, Duration, MissedTickBehavior};
use uuid::Uuid;

const TERMINAL_STREAM_POLL_INTERVAL_MS: u64 = 75;

#[derive(Clone)]
pub struct AppState {
    config: AppConfig,
    runtime: Arc<dyn SessionRuntime>,
    store: Arc<ControlStore>,
    events_tx: broadcast::Sender<LifecycleEvent>,
}

impl AppState {
    pub fn new(runtime: Arc<dyn SessionRuntime>, store_path: PathBuf) -> Result<Self, AppError> {
        Self::new_with_config(runtime, store_path, AppConfig::default())
    }

    pub fn new_with_config(
        runtime: Arc<dyn SessionRuntime>,
        store_path: PathBuf,
        config: AppConfig,
    ) -> Result<Self, AppError> {
        let (events_tx, _) = broadcast::channel(1024);
        let store = ControlStore::load(store_path)?;
        Ok(Self {
            config,
            runtime,
            store: Arc::new(store),
            events_tx,
        })
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AppConfig {
    pub terminal_renderer_v1_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub state: String,
    pub created_at: String,
    pub last_activity_at: String,
    pub kind: SessionKind,
    pub workspace: Workspace,
    pub managed_worktree: Option<ManagedWorktree>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub ready: bool,
    pub now: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSessionRequest {
    pub name: Option<String>,
    pub workspace_id: String,
    pub kind: SessionKind,
    pub managed_worktree_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterWorkspaceRequest {
    pub name: Option<String>,
    pub root_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateManagedWorktreeRequest {
    pub source_ref: String,
    pub branch_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleEvent {
    pub event_id: String,
    pub event_type: String,
    pub occurred_at: String,
    pub session_id: String,
}

#[derive(Debug)]
pub enum AppError {
    BadRequest { code: String, message: String },
    NotFound { code: String, message: String },
    Conflict { code: String, message: String },
    Runtime(String),
}

impl AppError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest {
            code: "invalid_request".to_string(),
            message: message.into(),
        }
    }

    pub fn not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::NotFound {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn conflict(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Conflict {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest { message, .. }
            | Self::NotFound { message, .. }
            | Self::Conflict { message, .. }
            | Self::Runtime(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::BadRequest { code, message } => (
                StatusCode::BAD_REQUEST,
                Json(ErrorEnvelope {
                    error: ErrorBody { code, message },
                }),
            )
                .into_response(),
            Self::NotFound { code, message } => (
                StatusCode::NOT_FOUND,
                Json(ErrorEnvelope {
                    error: ErrorBody { code, message },
                }),
            )
                .into_response(),
            Self::Conflict { code, message } => (
                StatusCode::CONFLICT,
                Json(ErrorEnvelope {
                    error: ErrorBody { code, message },
                }),
            )
                .into_response(),
            Self::Runtime(message) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorEnvelope {
                    error: ErrorBody {
                        code: "runtime_failure".to_string(),
                        message,
                    },
                }),
            )
                .into_response(),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

pub trait SessionRuntime: Send + Sync {
    fn create(&self, name: Option<&str>, cwd: &Path) -> Result<RuntimeSession, AppError>;
    fn list(&self) -> Result<Vec<RuntimeSession>, AppError>;
    fn terminate(&self, runtime_name: &str) -> Result<(), AppError>;
    fn capture_terminal(&self, runtime_name: &str) -> Result<TerminalSnapshot, AppError>;
    fn send_terminal_input(
        &self,
        runtime_name: &str,
        input: &TerminalInputRequest,
    ) -> Result<TerminalInputResponse, AppError>;
}

#[derive(Debug, Clone)]
pub struct RuntimeSession {
    pub runtime_name: String,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
}

pub struct TmuxRuntime;

impl SessionRuntime for TmuxRuntime {
    fn create(&self, name: Option<&str>, cwd: &Path) -> Result<RuntimeSession, AppError> {
        let base_name = name
            .filter(|value| !value.trim().is_empty())
            .map(sanitize_runtime_name)
            .unwrap_or_else(|| "session".to_string());
        let runtime_name = format!(
            "{}-{}",
            base_name,
            &Uuid::new_v4().simple().to_string()[..8]
        );

        let output = Command::new("tmux")
            .args([
                "new-session",
                "-d",
                "-s",
                &runtime_name,
                "-c",
                &cwd.to_string_lossy(),
            ])
            .output()
            .map_err(|error| {
                AppError::Runtime(format!("failed to execute tmux new-session: {error}"))
            })?;

        if !output.status.success() {
            return Err(AppError::Runtime(format!(
                "tmux new-session failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        self.list()?
            .into_iter()
            .find(|session| session.runtime_name == runtime_name)
            .ok_or_else(|| AppError::Runtime("created session missing from runtime".to_string()))
    }

    fn list(&self) -> Result<Vec<RuntimeSession>, AppError> {
        let output = Command::new("tmux")
            .args([
                "list-sessions",
                "-F",
                "#{session_name}|#{session_created}|#{session_activity}",
            ])
            .output()
            .map_err(|error| {
                AppError::Runtime(format!("failed to execute tmux list-sessions: {error}"))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("no server running") {
                return Ok(Vec::new());
            }
            return Err(AppError::Runtime(format!(
                "tmux list-sessions failed: {}",
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut sessions = Vec::new();
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() != 3 {
                continue;
            }

            let created_epoch = parts[1].parse::<i64>().unwrap_or(0);
            let activity_epoch = parts[2].parse::<i64>().unwrap_or(created_epoch);
            let created_at = Utc
                .timestamp_opt(created_epoch, 0)
                .single()
                .unwrap_or_else(Utc::now);
            let last_activity_at = Utc
                .timestamp_opt(activity_epoch, 0)
                .single()
                .unwrap_or(created_at);

            sessions.push(RuntimeSession {
                runtime_name: parts[0].to_string(),
                created_at,
                last_activity_at,
            });
        }

        Ok(sessions)
    }

    fn terminate(&self, runtime_name: &str) -> Result<(), AppError> {
        let output = Command::new("tmux")
            .args(["kill-session", "-t", runtime_name])
            .output()
            .map_err(|error| {
                AppError::Runtime(format!("failed to execute tmux kill-session: {error}"))
            })?;

        if !output.status.success() {
            return Err(AppError::Runtime(format!(
                "tmux kill-session failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        Ok(())
    }

    fn capture_terminal(&self, runtime_name: &str) -> Result<TerminalSnapshot, AppError> {
        let (rows, cols) = tmux_pane_size(runtime_name)?;
        let output = Command::new("tmux")
            .args(["capture-pane", "-e", "-p", "-S", "-", "-t", runtime_name])
            .output()
            .map_err(|error| {
                AppError::Runtime(format!("failed to execute tmux capture-pane: {error}"))
            })?;

        if !output.status.success() {
            return Err(AppError::Runtime(format!(
                "tmux capture-pane failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        let normalized = normalize_tmux_capture_stream(&output.stdout);
        let mut snapshot = build_tmux_terminal_snapshot(rows, cols, &normalized);
        if let Some(cursor) = tmux_cursor(runtime_name)? {
            snapshot.cursor = cursor;
        }

        Ok(snapshot)
    }

    fn send_terminal_input(
        &self,
        runtime_name: &str,
        input: &TerminalInputRequest,
    ) -> Result<TerminalInputResponse, AppError> {
        let mut literal_buffer = String::new();

        for event in &input.events {
            match event {
                TerminalInputEvent::Text { text } | TerminalInputEvent::Paste { text } => {
                    literal_buffer.push_str(text);
                }
                TerminalInputEvent::Key {
                    key,
                    ctrl,
                    alt,
                    shift: _,
                } => {
                    flush_tmux_literal(runtime_name, &mut literal_buffer)?;
                    let keys = tmux_key_sequence(key, *ctrl, *alt)?;
                    for key in keys {
                        let output = Command::new("tmux")
                            .args(["send-keys", "-t", runtime_name, &key])
                            .output()
                            .map_err(|error| {
                                AppError::Runtime(format!(
                                    "failed to execute tmux send-keys: {error}"
                                ))
                            })?;
                        ensure_tmux_success("send-keys", output)?;
                    }
                }
                TerminalInputEvent::Resize { rows, cols } => {
                    flush_tmux_literal(runtime_name, &mut literal_buffer)?;
                    let output = Command::new("tmux")
                        .args([
                            "resize-window",
                            "-t",
                            runtime_name,
                            "-x",
                            &cols.to_string(),
                            "-y",
                            &rows.to_string(),
                        ])
                        .output()
                        .map_err(|error| {
                            AppError::Runtime(format!(
                                "failed to execute tmux resize-window: {error}"
                            ))
                        })?;
                    ensure_tmux_success("resize-window", output)?;
                }
            }
        }

        flush_tmux_literal(runtime_name, &mut literal_buffer)?;

        Ok(TerminalInputResponse {
            accepted_events: input.events.len(),
        })
    }
}

fn flush_tmux_literal(runtime_name: &str, literal_buffer: &mut String) -> Result<(), AppError> {
    if literal_buffer.is_empty() {
        return Ok(());
    }

    let output = Command::new("tmux")
        .args([
            "send-keys",
            "-t",
            runtime_name,
            "-l",
            "--",
            literal_buffer.as_str(),
        ])
        .output()
        .map_err(|error| AppError::Runtime(format!("failed to execute tmux send-keys: {error}")))?;
    ensure_tmux_success("send-keys", output)?;
    literal_buffer.clear();
    Ok(())
}

fn ensure_tmux_success(command: &str, output: std::process::Output) -> Result<(), AppError> {
    if output.status.success() {
        return Ok(());
    }

    Err(AppError::Runtime(format!(
        "tmux {command} failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

fn normalize_tmux_capture_stream(bytes: &[u8]) -> Vec<u8> {
    let mut normalized = Vec::with_capacity(bytes.len() + bytes.len() / 16);
    let mut previous = None;

    for byte in bytes {
        if *byte == b'\n' && previous != Some(b'\r') {
            normalized.push(b'\r');
        }
        normalized.push(*byte);
        previous = Some(*byte);
    }

    normalized
}

fn build_tmux_terminal_snapshot(rows: u16, cols: u16, normalized: &[u8]) -> TerminalSnapshot {
    let mut visible = TerminalCore::new(rows, cols, 0);
    visible.ingest(normalized);
    let visible_snapshot = visible.snapshot();

    let full_rows = tmux_capture_line_count(normalized, rows);
    let mut full = TerminalCore::new(full_rows, cols, 0);
    full.ingest(normalized);
    let full_snapshot = full.snapshot();
    let (scrollback, plain_text) = split_snapshot_scrollback(&full_snapshot, rows);

    visible_snapshot.with_scrollback(scrollback, plain_text)
}

fn tmux_capture_line_count(normalized: &[u8], minimum_rows: u16) -> u16 {
    let line_count = String::from_utf8_lossy(normalized).lines().count().max(1);
    let bounded = line_count.max(usize::from(minimum_rows)).min(usize::from(u16::MAX));
    u16::try_from(bounded).unwrap_or(u16::MAX)
}

fn split_snapshot_scrollback(snapshot: &TerminalSnapshot, visible_rows: u16) -> (Vec<TerminalLine>, String) {
    let split_at = snapshot
        .lines
        .len()
        .saturating_sub(usize::from(visible_rows));
    let scrollback = reindex_terminal_lines(&snapshot.lines[..split_at]);
    (scrollback, snapshot.plain_text.clone())
}

fn reindex_terminal_lines(lines: &[TerminalLine]) -> Vec<TerminalLine> {
    lines
        .iter()
        .enumerate()
        .map(|(row, line)| TerminalLine {
            row: u16::try_from(row).unwrap_or(u16::MAX),
            wrapped: line.wrapped,
            cells: line.cells.clone(),
        })
        .collect()
}

fn tmux_pane_size(runtime_name: &str) -> Result<(u16, u16), AppError> {
    let output = Command::new("tmux")
        .args([
            "display-message",
            "-p",
            "-t",
            runtime_name,
            "#{pane_height}|#{pane_width}",
        ])
        .output()
        .map_err(|error| AppError::Runtime(format!("failed to execute tmux display-message: {error}")))?;
    ensure_tmux_success("display-message", output.clone())?;

    let raw = String::from_utf8_lossy(&output.stdout);
    let mut parts = raw.trim().split('|');
    let rows = parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| AppError::Runtime("failed parsing tmux pane height".to_string()))?;
    let cols = parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| AppError::Runtime("failed parsing tmux pane width".to_string()))?;
    Ok((rows, cols))
}

fn tmux_cursor(runtime_name: &str) -> Result<Option<TerminalCursor>, AppError> {
    let output = Command::new("tmux")
        .args([
            "display-message",
            "-p",
            "-t",
            runtime_name,
            "#{cursor_y}|#{cursor_x}|#{cursor_flag}",
        ])
        .output()
        .map_err(|error| AppError::Runtime(format!("failed to execute tmux display-message: {error}")))?;
    ensure_tmux_success("display-message", output.clone())?;

    let raw = String::from_utf8_lossy(&output.stdout);
    let mut parts = raw.trim().split('|');
    let row = match parts.next().and_then(|value| value.parse::<u16>().ok()) {
        Some(row) => row,
        None => return Ok(None),
    };
    let col = match parts.next().and_then(|value| value.parse::<u16>().ok()) {
        Some(col) => col,
        None => return Ok(None),
    };
    let visible = parts.next() == Some("1");

    Ok(Some(TerminalCursor { row, col, visible }))
}

fn tmux_key_sequence(
    key: &terminal::TerminalKey,
    ctrl: bool,
    alt: bool,
) -> Result<Vec<String>, AppError> {
    let mut keys = Vec::new();

    if alt {
        keys.push("Escape".to_string());
    }

    match key {
        terminal::TerminalKey::Named { key } => {
            if ctrl {
                return match key {
                    terminal::TerminalNamedKey::ArrowUp => Ok(vec!["C-Up".to_string()]),
                    terminal::TerminalNamedKey::ArrowDown => Ok(vec!["C-Down".to_string()]),
                    terminal::TerminalNamedKey::ArrowLeft => Ok(vec!["C-Left".to_string()]),
                    terminal::TerminalNamedKey::ArrowRight => Ok(vec!["C-Right".to_string()]),
                    terminal::TerminalNamedKey::Tab => Ok(vec!["C-i".to_string()]),
                    terminal::TerminalNamedKey::Enter => Ok(vec!["C-m".to_string()]),
                    terminal::TerminalNamedKey::Escape => Ok(vec!["C-[".to_string()]),
                    terminal::TerminalNamedKey::Ctrl => Err(AppError::bad_request(
                        "bare ctrl key events are not supported; send a ctrl character chord",
                    )),
                };
            }

            let mapped = match key {
                terminal::TerminalNamedKey::Ctrl => {
                    return Err(AppError::bad_request(
                        "bare ctrl key events are not supported; send a ctrl character chord",
                    ));
                }
                terminal::TerminalNamedKey::Escape => "Escape",
                terminal::TerminalNamedKey::Tab => "Tab",
                terminal::TerminalNamedKey::ArrowUp => "Up",
                terminal::TerminalNamedKey::ArrowDown => "Down",
                terminal::TerminalNamedKey::ArrowLeft => "Left",
                terminal::TerminalNamedKey::ArrowRight => "Right",
                terminal::TerminalNamedKey::Enter => "Enter",
            };
            keys.push(mapped.to_string());
        }
        terminal::TerminalKey::Character { text } => {
            let mut chars = text.chars();
            let ch = chars.next().ok_or_else(|| {
                AppError::bad_request("character key events require one character")
            })?;
            if chars.next().is_some() {
                return Err(AppError::bad_request(
                    "character key events require exactly one character",
                ));
            }

            if ctrl {
                if !ch.is_ascii() {
                    return Err(AppError::bad_request(
                        "ctrl character chords currently require ASCII input",
                    ));
                }
                keys.push(format!("C-{}", ch.to_ascii_lowercase()));
            } else {
                keys.push(text.clone());
            }
        }
    }

    Ok(keys)
}

pub fn build_router(state: AppState) -> Router {
    let mut app = Router::new()
        .route("/health", get(get_health))
        .route("/app", get(shell::shell_entry))
        .route("/app/sessions/:session_id", get(shell::shell_session_entry))
        .route("/app/assets/*asset", get(shell::shell_asset))
        .route("/workspaces", post(register_workspace).get(list_workspaces))
        .route(
            "/workspaces/:workspace_id/source-refs",
            get(list_source_refs),
        )
        .route(
            "/workspaces/:workspace_id/worktrees",
            post(create_managed_worktree).get(list_managed_worktrees),
        )
        .route("/sessions", post(create_session).get(list_sessions))
        .route(
            "/sessions/:session_id",
            get(get_session).delete(terminate_session),
        )
        .route("/ws/events", get(ws_events));

    if state.config.terminal_renderer_v1_enabled {
        app = app
            .route("/sessions/:session_id/terminal", get(get_terminal_surface))
            .route(
                "/sessions/:session_id/terminal/stream",
                get(ws_terminal_stream),
            )
            .route(
                "/sessions/:session_id/terminal/input",
                post(post_terminal_input),
            );
    }

    app.with_state(state)
}

async fn get_health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        ready: true,
        now: now_rfc3339(),
    })
}

async fn register_workspace(
    State(state): State<AppState>,
    Json(payload): Json<RegisterWorkspaceRequest>,
) -> AppResult<Json<Workspace>> {
    let (resolved_root, kind) = resolve_workspace_root(&payload.root_path)?;
    let name = payload.name.unwrap_or_else(|| basename_for_path(&resolved_root));

    let workspace = state.store.insert_workspace(NewWorkspace {
        name,
        root_path: resolved_root.to_string_lossy().into_owned(),
        kind,
    })?;
    Ok(Json(workspace))
}

async fn list_workspaces(State(state): State<AppState>) -> AppResult<Json<Vec<Workspace>>> {
    Ok(Json(state.store.list_workspaces()?))
}

async fn list_source_refs(
    State(state): State<AppState>,
    AxumPath(workspace_id): AxumPath<String>,
) -> AppResult<Json<Vec<SourceRef>>> {
    let workspace = workspace_by_id(&state, &workspace_id)?;
    if workspace.kind == WorkspaceKind::None {
        return Ok(Json(Vec::new()));
    }

    Ok(Json(discover_source_refs(&workspace)?))
}

async fn create_managed_worktree(
    State(state): State<AppState>,
    AxumPath(workspace_id): AxumPath<String>,
    Json(payload): Json<CreateManagedWorktreeRequest>,
) -> AppResult<Json<ManagedWorktree>> {
    let workspace = workspace_by_id(&state, &workspace_id)?;
    ensure_git_workspace(&workspace)?;

    let source_ref = payload.source_ref.trim();
    if source_ref.is_empty() {
        return Err(AppError::bad_request("source_ref is required"));
    }

    let branch_name = payload.branch_name.trim();
    if branch_name.is_empty() {
        return Err(AppError::bad_request("branch_name is required"));
    }

    let path = managed_worktree_path(&workspace, branch_name)?;
    if path.exists() {
        return Err(AppError::conflict(
            "managed_worktree_path_exists",
            format!("managed worktree path already exists: {}", path.display()),
        ));
    }

    let existing = state.store.list_managed_worktrees(&workspace.id)?;
    if existing.iter().any(|worktree| worktree.branch_name == branch_name) {
        return Err(AppError::conflict(
            "managed_branch_exists",
            format!(
                "managed worktree branch already exists in workspace: {branch_name}"
            ),
        ));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::Runtime(format!("failed creating managed worktree directory: {error}"))
        })?;
    }

    let output = Command::new("git")
        .current_dir(&workspace.root_path)
        .args([
            "worktree",
            "add",
            "-b",
            branch_name,
            &path.to_string_lossy(),
            source_ref,
        ])
        .output()
        .map_err(|error| AppError::Runtime(format!("failed to execute git worktree add: {error}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.contains("already exists") {
            return Err(AppError::conflict(
                "branch_name_conflict",
                format!("branch_name is not new: {branch_name}"),
            ));
        }
        return Err(AppError::bad_request(format!(
            "failed creating managed worktree: {stderr}"
        )));
    }

    let worktree = state.store.insert_managed_worktree(NewManagedWorktree {
        workspace_id: workspace.id,
        branch_name: branch_name.to_string(),
        source_ref: source_ref.to_string(),
        path: path.to_string_lossy().into_owned(),
    })?;

    Ok(Json(worktree))
}

async fn list_managed_worktrees(
    State(state): State<AppState>,
    AxumPath(workspace_id): AxumPath<String>,
) -> AppResult<Json<Vec<ManagedWorktree>>> {
    let workspace = workspace_by_id(&state, &workspace_id)?;
    if workspace.kind == WorkspaceKind::None {
        return Ok(Json(Vec::new()));
    }

    Ok(Json(state.store.list_managed_worktrees(&workspace_id)?))
}

async fn create_session(
    State(state): State<AppState>,
    Json(payload): Json<CreateSessionRequest>,
) -> AppResult<Json<Session>> {
    let workspace = workspace_by_id(&state, &payload.workspace_id)?;
    let managed_worktree = match payload.kind {
        SessionKind::Local => None,
        SessionKind::Worktree => {
            ensure_git_workspace(&workspace)?;
            let managed_worktree_id = payload.managed_worktree_id.as_deref().ok_or_else(|| {
                AppError::bad_request("managed_worktree_id is required for worktree sessions")
            })?;
            let managed_worktree = managed_worktree_by_id(&state, managed_worktree_id)?;
            if managed_worktree.workspace_id != workspace.id {
                return Err(AppError::bad_request(
                    "managed worktree does not belong to the requested workspace",
                ));
            }
            Some(managed_worktree)
        }
    };

    let cwd = managed_worktree
        .as_ref()
        .map(|worktree| PathBuf::from(&worktree.path))
        .unwrap_or_else(|| PathBuf::from(&workspace.root_path));
    let runtime_session = state.runtime.create(payload.name.as_deref(), &cwd)?;
    let session_name = payload
        .name
        .unwrap_or_else(|| runtime_session.runtime_name.clone());
    let session_id = Uuid::new_v4().to_string();

    let stored = state.store.insert_session(NewStoredSession {
        id: session_id.clone(),
        name: session_name.clone(),
        runtime_name: runtime_session.runtime_name.clone(),
        kind: payload.kind,
        workspace_id: workspace.id.clone(),
        managed_worktree_id: managed_worktree.as_ref().map(|worktree| worktree.id.clone()),
    })?;

    let response = build_session_response(
        &stored,
        &runtime_session,
        &workspace,
        managed_worktree.as_ref(),
    );

    let _ = state.events_tx.send(LifecycleEvent {
        event_id: Uuid::new_v4().to_string(),
        event_type: "session.created".to_string(),
        occurred_at: now_rfc3339(),
        session_id,
    });

    Ok(Json(response))
}

async fn list_sessions(State(state): State<AppState>) -> AppResult<Json<Vec<Session>>> {
    let runtime_index: HashMap<String, RuntimeSession> = state
        .runtime
        .list()?
        .into_iter()
        .map(|session| (session.runtime_name.clone(), session))
        .collect();
    let workspaces: HashMap<String, Workspace> = state
        .store
        .list_workspaces()?
        .into_iter()
        .map(|workspace| (workspace.id.clone(), workspace))
        .collect();
    let worktrees: HashMap<String, ManagedWorktree> = state
        .store
        .list_all_managed_worktrees()?
        .into_iter()
        .map(|worktree| (worktree.id.clone(), worktree))
        .collect();

    let mut sessions = Vec::new();
    for stored in state.store.list_sessions()? {
        let Some(runtime) = runtime_index.get(&stored.runtime_name) else {
            continue;
        };
        let workspace = workspaces.get(&stored.workspace_id).cloned().ok_or_else(|| {
            AppError::Runtime(format!(
                "session {} references missing workspace {}",
                stored.id, stored.workspace_id
            ))
        })?;
        let managed_worktree = match stored.managed_worktree_id.as_ref() {
            Some(managed_worktree_id) => Some(worktrees.get(managed_worktree_id).cloned().ok_or_else(
                || {
                    AppError::Runtime(format!(
                        "session {} references missing managed worktree {}",
                        stored.id, managed_worktree_id
                    ))
                },
            )?),
            None => None,
        };

        sessions.push((
            runtime.created_at,
            build_session_response(&stored, runtime, &workspace, managed_worktree.as_ref()),
        ));
    }
    sessions.sort_by(|left, right| right.0.cmp(&left.0));

    Ok(Json(
        sessions
            .into_iter()
            .map(|(_, session)| session)
            .collect(),
    ))
}

async fn get_session(
    State(state): State<AppState>,
    AxumPath(session_id): AxumPath<String>,
) -> AppResult<Json<Session>> {
    let stored = stored_session_by_id(&state, &session_id)?;
    let runtime = state
        .runtime
        .list()?
        .into_iter()
        .find(|session| session.runtime_name == stored.runtime_name)
        .ok_or_else(|| session_not_found_error(&session_id))?;
    let workspace = workspace_by_id(&state, &stored.workspace_id)?;
    let managed_worktree = match stored.managed_worktree_id.as_deref() {
        Some(managed_worktree_id) => Some(managed_worktree_by_id(&state, managed_worktree_id)?),
        None => None,
    };

    Ok(Json(build_session_response(
        &stored,
        &runtime,
        &workspace,
        managed_worktree.as_ref(),
    )))
}

async fn terminate_session(
    State(state): State<AppState>,
    AxumPath(session_id): AxumPath<String>,
) -> AppResult<StatusCode> {
    let stored = stored_session_by_id(&state, &session_id)?;

    let runtime_sessions = state.runtime.list()?;
    if runtime_sessions
        .iter()
        .all(|session| session.runtime_name != stored.runtime_name)
    {
        let _ = state.store.remove_session(&session_id)?;
        return Err(session_not_found_error(&session_id));
    }

    state.runtime.terminate(&stored.runtime_name)?;
    let _ = state.store.remove_session(&session_id)?;

    let _ = state.events_tx.send(LifecycleEvent {
        event_id: Uuid::new_v4().to_string(),
        event_type: "session.terminated".to_string(),
        occurred_at: now_rfc3339(),
        session_id,
    });

    Ok(StatusCode::NO_CONTENT)
}

async fn ws_events(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    let rx = state.events_tx.subscribe();
    ws.on_upgrade(move |socket| stream_events(socket, rx))
}

async fn get_terminal_surface(
    State(state): State<AppState>,
    AxumPath(session_id): AxumPath<String>,
) -> AppResult<Json<TerminalSurfaceState>> {
    Ok(Json(capture_terminal_surface_state(&state, &session_id)?))
}

async fn ws_terminal_stream(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    AxumPath(session_id): AxumPath<String>,
) -> AppResult<impl IntoResponse> {
    let stored = stored_session_by_id(&state, &session_id)?;
    Ok(ws.on_upgrade(move |socket| {
        stream_terminal(socket, state, session_id, stored.runtime_name)
    }))
}

async fn post_terminal_input(
    State(state): State<AppState>,
    AxumPath(session_id): AxumPath<String>,
    Json(payload): Json<TerminalInputRequest>,
) -> AppResult<Json<TerminalInputResponse>> {
    let stored = stored_session_by_id(&state, &session_id)?;
    let response = state
        .runtime
        .send_terminal_input(&stored.runtime_name, &payload)?;
    Ok(Json(response))
}

async fn stream_events(mut socket: WebSocket, mut rx: broadcast::Receiver<LifecycleEvent>) {
    loop {
        match rx.recv().await {
            Ok(event) => {
                if let Ok(payload) = serde_json::to_string(&event)
                    && socket.send(Message::Text(payload.into())).await.is_err()
                {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}

#[derive(Clone)]
enum TerminalStreamUpdate {
    Snapshot(TerminalSnapshot),
    Closed,
}

fn capture_terminal_surface_state(
    state: &AppState,
    session_id: &str,
) -> Result<TerminalSurfaceState, AppError> {
    let stored = stored_session_by_id(state, session_id)?;
    let snapshot = state.runtime.capture_terminal(&stored.runtime_name)?;
    Ok(TerminalSurfaceState::baseline(session_id.to_string(), snapshot))
}

fn capture_stream_terminal_snapshot(
    state: &AppState,
    session_id: &str,
    runtime_name: &str,
) -> Result<TerminalSnapshot, AppError> {
    let stored = stored_session_by_id(state, session_id)?;
    if stored.runtime_name != runtime_name {
        return Err(session_not_found_error(session_id));
    }

    state.runtime.capture_terminal(runtime_name)
}

async fn stream_terminal(
    mut socket: WebSocket,
    state: AppState,
    session_id: String,
    runtime_name: String,
) {
    let Ok(initial_snapshot) = capture_stream_terminal_snapshot(&state, &session_id, &runtime_name)
    else {
        let _ = socket.close().await;
        return;
    };

    let mut sequence = 1_u64;
    if send_terminal_stream_frame(&mut socket, &initial_snapshot.diff_frame(&session_id, sequence, None))
        .await
        .is_err()
    {
        return;
    }

    let (updates_tx, mut updates_rx) = watch::channel(TerminalStreamUpdate::Snapshot(initial_snapshot.clone()));
    let producer_state = state.clone();
    let producer_session_id = session_id.clone();
    let producer_runtime_name = runtime_name.clone();
    let producer = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(TERMINAL_STREAM_POLL_INTERVAL_MS));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let mut latest_snapshot = initial_snapshot;
        loop {
            interval.tick().await;
            match capture_stream_terminal_snapshot(
                &producer_state,
                &producer_session_id,
                &producer_runtime_name,
            ) {
                Ok(snapshot) => {
                    if snapshot != latest_snapshot {
                        latest_snapshot = snapshot.clone();
                        if updates_tx.send(TerminalStreamUpdate::Snapshot(snapshot)).is_err() {
                            break;
                        }
                    }
                }
                Err(_) => {
                    let _ = updates_tx.send(TerminalStreamUpdate::Closed);
                    break;
                }
            }
        }
    });

    let initial_update = updates_rx.borrow().clone();
    let mut sent_snapshot = match initial_update {
        TerminalStreamUpdate::Snapshot(snapshot) => snapshot,
        TerminalStreamUpdate::Closed => {
            let _ = socket.close().await;
            producer.abort();
            return;
        }
    };

    loop {
        if updates_rx.changed().await.is_err() {
            break;
        }

        let update = updates_rx.borrow().clone();
        match update {
            TerminalStreamUpdate::Snapshot(snapshot) => {
                sequence += 1;
                let frame = snapshot.diff_frame(&session_id, sequence, Some(&sent_snapshot));
                if send_terminal_stream_frame(&mut socket, &frame).await.is_err() {
                    break;
                }
                sent_snapshot = snapshot;
            }
            TerminalStreamUpdate::Closed => {
                let _ = socket.close().await;
                break;
            }
        }
    }

    producer.abort();
}

async fn send_terminal_stream_frame(
    socket: &mut WebSocket,
    frame: &TerminalStreamFrame,
) -> Result<(), axum::Error> {
    let payload = serde_json::to_string(frame)
        .map_err(|error| axum::Error::new(std::io::Error::other(error.to_string())))?;
    socket.send(Message::Text(payload.into())).await
}

fn build_session_response(
    stored: &StoredSession,
    runtime: &RuntimeSession,
    workspace: &Workspace,
    managed_worktree: Option<&ManagedWorktree>,
) -> Session {
    Session {
        id: stored.id.clone(),
        name: stored.name.clone(),
        state: "running".to_string(),
        created_at: to_rfc3339_utc(runtime.created_at),
        last_activity_at: to_rfc3339_utc(runtime.last_activity_at),
        kind: stored.kind,
        workspace: workspace.clone(),
        managed_worktree: managed_worktree.cloned(),
    }
}

fn workspace_by_id(state: &AppState, workspace_id: &str) -> Result<Workspace, AppError> {
    state
        .store
        .get_workspace(workspace_id)?
        .ok_or_else(|| workspace_not_found_error(workspace_id))
}

fn managed_worktree_by_id(
    state: &AppState,
    managed_worktree_id: &str,
) -> Result<ManagedWorktree, AppError> {
    state
        .store
        .get_managed_worktree(managed_worktree_id)?
        .ok_or_else(|| managed_worktree_not_found_error(managed_worktree_id))
}

fn stored_session_by_id(state: &AppState, session_id: &str) -> Result<StoredSession, AppError> {
    state
        .store
        .get_session(session_id)?
        .ok_or_else(|| session_not_found_error(session_id))
}

fn session_not_found_error(session_id: &str) -> AppError {
    AppError::not_found(
        "session_not_found",
        format!("session not found: {session_id}"),
    )
}

fn workspace_not_found_error(workspace_id: &str) -> AppError {
    AppError::not_found(
        "workspace_not_found",
        format!("workspace not found: {workspace_id}"),
    )
}

fn managed_worktree_not_found_error(managed_worktree_id: &str) -> AppError {
    AppError::not_found(
        "managed_worktree_not_found",
        format!("managed worktree not found: {managed_worktree_id}"),
    )
}

fn ensure_git_workspace(workspace: &Workspace) -> Result<(), AppError> {
    if workspace.kind == WorkspaceKind::Git {
        return Ok(());
    }

    Err(AppError::bad_request(
        "managed worktrees and worktree sessions are only supported for git workspaces",
    ))
}

fn resolve_workspace_root(raw_root: &str) -> Result<(PathBuf, WorkspaceKind), AppError> {
    let trimmed = raw_root.trim();
    if trimmed.is_empty() {
        return Err(AppError::bad_request("root_path is required"));
    }

    let canonical = PathBuf::from(trimmed).canonicalize().map_err(|error| {
        AppError::bad_request(format!("workspace root_path is invalid: {error}"))
    })?;
    if !canonical.is_dir() {
        return Err(AppError::bad_request(
            "workspace root_path must resolve to a directory",
        ));
    }

    if let Some(git_root) = git_show_toplevel(&canonical)? {
        return Ok((git_root, WorkspaceKind::Git));
    }

    Ok((canonical, WorkspaceKind::None))
}

fn git_show_toplevel(path: &Path) -> Result<Option<PathBuf>, AppError> {
    let output = Command::new("git")
        .current_dir(path)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|error| AppError::Runtime(format!("failed to execute git rev-parse: {error}")))?;

    if !output.status.success() {
        return Ok(None);
    }

    let top_level = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let resolved = PathBuf::from(top_level).canonicalize().map_err(|error| {
        AppError::Runtime(format!("failed canonicalizing git workspace root: {error}"))
    })?;
    Ok(Some(resolved))
}

fn discover_source_refs(workspace: &Workspace) -> Result<Vec<SourceRef>, AppError> {
    let output = Command::new("git")
        .current_dir(&workspace.root_path)
        .args([
            "for-each-ref",
            "--format=%(refname:short)|%(refname)",
            "refs/heads",
            "refs/remotes",
        ])
        .output()
        .map_err(|error| {
            AppError::Runtime(format!("failed to execute git for-each-ref: {error}"))
        })?;

    if !output.status.success() {
        return Err(AppError::Runtime(format!(
            "git for-each-ref failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    let mut refs = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Some((short_name, full_name)) = line.split_once('|') else {
            continue;
        };
        if full_name.ends_with("/HEAD") {
            continue;
        }

        let kind = if full_name.starts_with("refs/heads/") {
            SourceRefKind::LocalBranch
        } else {
            SourceRefKind::RemoteTrackingBranch
        };
        refs.push(SourceRef {
            name: short_name.to_string(),
            kind,
        });
    }

    refs.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(refs)
}

fn managed_worktree_path(workspace: &Workspace, branch_name: &str) -> Result<PathBuf, AppError> {
    let workspace_root = Path::new(&workspace.root_path);
    let workspace_parent = workspace_root.parent().ok_or_else(|| {
        AppError::bad_request("workspace root must have a parent directory for worktree storage")
    })?;
    let workspace_slug = slugify(&basename_for_path(workspace_root));
    let branch_slug = slugify(branch_name);

    Ok(workspace_parent
        .join(".amux-worktrees")
        .join(workspace_slug)
        .join(branch_slug))
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "workspace".to_string()
    } else {
        trimmed
    }
}

fn sanitize_runtime_name(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        }
    }
    if out.is_empty() {
        "session".to_string()
    } else {
        out
    }
}

pub fn default_store_path(data_dir: &Path) -> PathBuf {
    data_dir.join("control.sqlite")
}

pub fn now_rfc3339() -> String {
    to_rfc3339_utc(Utc::now())
}

pub fn to_rfc3339_utc(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use futures_util::StreamExt;
    use http_body_util::BodyExt;
    use reqwest::Client;
    use std::net::SocketAddr;
    use std::sync::Mutex;
    use tempfile::{TempDir, tempdir};
    use terminal::{
        EscapeSequenceMetrics, TerminalCell, TerminalColor, TerminalKey, TerminalLine,
        TerminalModes, TerminalNamedKey, TerminalStreamFrame,
    };
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;
    use tokio_tungstenite::connect_async;
    use tower::ServiceExt;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct CreateCall {
        runtime_name: String,
        cwd: PathBuf,
    }

    #[derive(Default)]
    struct MockRuntime {
        sessions: Mutex<HashMap<String, RuntimeSession>>,
        snapshots: Mutex<HashMap<String, TerminalSnapshot>>,
        inputs: Mutex<Vec<TerminalInputRequest>>,
        create_calls: Mutex<Vec<CreateCall>>,
    }

    impl SessionRuntime for MockRuntime {
        fn create(&self, name: Option<&str>, cwd: &Path) -> Result<RuntimeSession, AppError> {
            let runtime_name = name.unwrap_or("session").to_string();
            let now = Utc::now();
            let session = RuntimeSession {
                runtime_name: runtime_name.clone(),
                created_at: now,
                last_activity_at: now,
            };
            self.sessions
                .lock()
                .expect("lock")
                .insert(runtime_name.clone(), session.clone());
            self.snapshots
                .lock()
                .expect("lock")
                .insert(runtime_name.clone(), sample_snapshot());
            self.create_calls.lock().expect("lock").push(CreateCall {
                runtime_name,
                cwd: cwd.to_path_buf(),
            });
            Ok(session)
        }

        fn list(&self) -> Result<Vec<RuntimeSession>, AppError> {
            Ok(self
                .sessions
                .lock()
                .expect("lock")
                .values()
                .cloned()
                .collect())
        }

        fn terminate(&self, runtime_name: &str) -> Result<(), AppError> {
            self.sessions
                .lock()
                .expect("lock")
                .remove(runtime_name)
                .ok_or_else(|| AppError::not_found("session_not_found", "session not found"))?;
            self.snapshots.lock().expect("lock").remove(runtime_name);
            Ok(())
        }

        fn capture_terminal(&self, runtime_name: &str) -> Result<TerminalSnapshot, AppError> {
            self.snapshots
                .lock()
                .expect("lock")
                .get(runtime_name)
                .cloned()
                .ok_or_else(|| AppError::not_found("session_not_found", "session not found"))
        }

        fn send_terminal_input(
            &self,
            runtime_name: &str,
            input: &TerminalInputRequest,
        ) -> Result<TerminalInputResponse, AppError> {
            if !self
                .sessions
                .lock()
                .expect("lock")
                .contains_key(runtime_name)
            {
                return Err(AppError::not_found("session_not_found", "session not found"));
            }

            self.inputs.lock().expect("lock").push(input.clone());
            Ok(TerminalInputResponse {
                accepted_events: input.events.len(),
            })
        }
    }

    fn test_state(runtime: Arc<dyn SessionRuntime>, store_path: PathBuf) -> AppState {
        AppState::new(runtime, store_path).expect("state")
    }

    fn terminal_enabled_state(runtime: Arc<dyn SessionRuntime>, store_path: PathBuf) -> AppState {
        AppState::new_with_config(
            runtime,
            store_path,
            AppConfig {
                terminal_renderer_v1_enabled: true,
            },
        )
        .expect("state")
    }

    fn sample_snapshot() -> TerminalSnapshot {
        TerminalSnapshot {
            rows: 2,
            cols: 4,
            cursor: TerminalCursor {
                row: 1,
                col: 2,
                visible: true,
            },
            modes: TerminalModes {
                application_cursor: false,
                application_keypad: false,
                bracketed_paste: true,
                alternate_screen: false,
            },
            escape_sequence_metrics: EscapeSequenceMetrics {
                print: 3,
                execute: 1,
                csi: 1,
                esc: 0,
                osc: 0,
                dcs: 0,
            },
            lines: vec![
                TerminalLine {
                    row: 0,
                    wrapped: false,
                    cells: vec![
                        TerminalCell {
                            column: 0,
                            text: "a".to_string(),
                            column_span: 1,
                            unicode_width: 1,
                            grapheme_count: 1,
                            is_wide: false,
                            is_wide_continuation: false,
                            foreground: TerminalColor::Default,
                            background: TerminalColor::Default,
                            bold: false,
                            italic: false,
                            underline: false,
                            inverse: false,
                        },
                        TerminalCell {
                            column: 1,
                            text: "b".to_string(),
                            column_span: 1,
                            unicode_width: 1,
                            grapheme_count: 1,
                            is_wide: false,
                            is_wide_continuation: false,
                            foreground: TerminalColor::Indexed(2),
                            background: TerminalColor::Default,
                            bold: true,
                            italic: false,
                            underline: false,
                            inverse: false,
                        },
                    ],
                },
                TerminalLine {
                    row: 1,
                    wrapped: false,
                    cells: vec![TerminalCell {
                        column: 0,
                        text: "😀".to_string(),
                        column_span: 2,
                        unicode_width: 2,
                        grapheme_count: 1,
                        is_wide: true,
                        is_wide_continuation: false,
                        foreground: TerminalColor::Default,
                        background: TerminalColor::Default,
                        bold: false,
                        italic: false,
                        underline: false,
                        inverse: false,
                    }],
                },
            ],
            scrollback: Vec::new(),
            plain_text: "ab\n😀".to_string(),
        }
    }

    fn sample_snapshot_with_scrollback() -> TerminalSnapshot {
        let mut snapshot = sample_snapshot();
        snapshot.scrollback = vec![TerminalLine {
            row: 0,
            wrapped: false,
            cells: vec![TerminalCell {
                column: 0,
                text: "$".to_string(),
                column_span: 1,
                unicode_width: 1,
                grapheme_count: 1,
                is_wide: false,
                is_wide_continuation: false,
                foreground: TerminalColor::Default,
                background: TerminalColor::Default,
                bold: false,
                italic: false,
                underline: false,
                inverse: false,
            }],
        }];
        snapshot.plain_text = "$\nab\n😀".to_string();
        snapshot
    }

    fn snapshot_with_text(first: &str, second: &str) -> TerminalSnapshot {
        let mut snapshot = sample_snapshot();
        apply_text_to_line(&mut snapshot.lines[0], first);
        apply_text_to_line(&mut snapshot.lines[1], second);
        snapshot.plain_text = format!("{first}\n{second}");
        snapshot
    }

    fn apply_text_to_line(line: &mut TerminalLine, text: &str) {
        line.cells = text
            .chars()
            .enumerate()
            .map(|(index, ch)| TerminalCell {
                column: u16::try_from(index).unwrap_or(u16::MAX),
                text: ch.to_string(),
                column_span: 1,
                unicode_width: 1,
                grapheme_count: 1,
                is_wide: false,
                is_wide_continuation: false,
                foreground: TerminalColor::Default,
                background: TerminalColor::Default,
                bold: false,
                italic: false,
                underline: false,
                inverse: false,
            })
            .collect();
    }

    fn decode_json<T: for<'de> Deserialize<'de>, B: AsRef<[u8]>>(bytes: B) -> T {
        serde_json::from_slice(bytes.as_ref()).expect("json")
    }

    async fn create_workspace_request(
        app: &Router,
        root_path: &Path,
        name: &str,
    ) -> Workspace {
        let request = Request::builder()
            .method("POST")
            .uri("/workspaces")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "name": name,
                    "root_path": root_path.to_string_lossy(),
                }))
                .expect("json"),
            ))
            .expect("workspace req");
        let response = app.clone().oneshot(request).await.expect("workspace resp");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json(
            response
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        )
    }

    async fn create_local_session_request(app: &Router, workspace_id: &str, name: &str) -> Session {
        let request = Request::builder()
            .method("POST")
            .uri("/sessions")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "name": name,
                    "workspace_id": workspace_id,
                    "kind": "local",
                }))
                .expect("json"),
            ))
            .expect("session req");
        let response = app.clone().oneshot(request).await.expect("session resp");
        assert_eq!(response.status(), StatusCode::OK);
        decode_json(
            response
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        )
    }

    async fn spawn_server(app: Router) -> (SocketAddr, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        (addr, handle)
    }

    fn init_git_workspace() -> (TempDir, TempDir) {
        let repo_parent = tempdir().expect("repo tempdir");
        let remote_parent = tempdir().expect("remote tempdir");
        let repo_path = repo_parent.path().join("workspace");
        let remote_path = remote_parent.path().join("origin.git");
        fs::create_dir_all(&repo_path).expect("mkdir repo");

        run_git(&repo_path, ["init", "-b", "main"]);
        fs::write(repo_path.join("README.md"), "hello\n").expect("write readme");
        run_git(&repo_path, ["add", "README.md"]);
        run_git_with_commit_env(&repo_path, ["commit", "-m", "init"]);
        run_git(&repo_path, ["branch", "local-only"]);
        run_git(remote_parent.path(), ["init", "--bare", remote_path.to_str().expect("utf8")]);
        run_git(&repo_path, ["remote", "add", "origin", remote_path.to_str().expect("utf8")]);
        run_git(&repo_path, ["push", "-u", "origin", "main"]);

        (repo_parent, remote_parent)
    }

    fn run_git<I, S>(cwd: &Path, args: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let output = Command::new("git")
            .current_dir(cwd)
            .args(args)
            .output()
            .expect("git command");
        assert!(
            output.status.success(),
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn run_git_with_commit_env<I, S>(cwd: &Path, args: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let output = Command::new("git")
            .current_dir(cwd)
            .args(args)
            .env("GIT_AUTHOR_NAME", "AMUX Tests")
            .env("GIT_AUTHOR_EMAIL", "tests@example.com")
            .env("GIT_COMMITTER_NAME", "AMUX Tests")
            .env("GIT_COMMITTER_EMAIL", "tests@example.com")
            .output()
            .expect("git command");
        assert!(
            output.status.success(),
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn tmux_capture_normalization_inserts_carriage_returns_before_linefeeds() {
        let normalized = normalize_tmux_capture_stream(b"prompt 1\nprompt 2\n");
        assert_eq!(normalized, b"prompt 1\r\nprompt 2\r\n");

        let already_normalized = normalize_tmux_capture_stream(b"prompt 1\r\nprompt 2\r\n");
        assert_eq!(already_normalized, b"prompt 1\r\nprompt 2\r\n");
    }

    #[tokio::test]
    async fn validate_workspace_scoped_session_lifecycle_flow() {
        let temp = tempdir().expect("tempdir");
        let runtime: Arc<dyn SessionRuntime> = Arc::new(MockRuntime::default());
        let state = test_state(runtime, temp.path().join("control.sqlite"));
        let app = build_router(state.clone());

        let workspace = create_workspace_request(&app, temp.path(), "sandbox").await;
        assert_eq!(workspace.kind, WorkspaceKind::None);

        let create_req = Request::builder()
            .method("POST")
            .uri("/sessions")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "name": "alpha",
                    "workspace_id": workspace.id,
                    "kind": "local",
                }))
                .expect("json"),
            ))
            .expect("create req");
        let create_resp = app.clone().oneshot(create_req).await.expect("create resp");
        assert_eq!(create_resp.status(), StatusCode::OK);
        let created: Session = decode_json(
            create_resp
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        );
        assert_eq!(created.kind, SessionKind::Local);
        assert_eq!(created.workspace.id, workspace.id);
        assert!(created.managed_worktree.is_none());

        let list_req = Request::builder()
            .method("GET")
            .uri("/sessions")
            .body(Body::empty())
            .expect("list req");
        let list_resp = app.clone().oneshot(list_req).await.expect("list resp");
        assert_eq!(list_resp.status(), StatusCode::OK);
        let sessions: Vec<Session> = decode_json(
            list_resp
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        );
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, created.id);

        let get_req = Request::builder()
            .method("GET")
            .uri(format!("/sessions/{}", created.id))
            .body(Body::empty())
            .expect("get req");
        let get_resp = app.clone().oneshot(get_req).await.expect("get resp");
        assert_eq!(get_resp.status(), StatusCode::OK);

        let delete_req = Request::builder()
            .method("DELETE")
            .uri(format!("/sessions/{}", created.id))
            .body(Body::empty())
            .expect("delete req");
        let delete_resp = app.clone().oneshot(delete_req).await.expect("delete resp");
        assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

        let missing_req = Request::builder()
            .method("GET")
            .uri(format!("/sessions/{}", created.id))
            .body(Body::empty())
            .expect("missing req");
        let missing_resp = app.oneshot(missing_req).await.expect("missing resp");
        assert_eq!(missing_resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn validate_git_workspace_source_refs_and_managed_worktrees() {
        let temp = tempdir().expect("tempdir");
        let runtime: Arc<dyn SessionRuntime> = Arc::new(MockRuntime::default());
        let app = build_router(test_state(runtime, temp.path().join("control.sqlite")));
        let (repo_parent, _remote_parent) = init_git_workspace();
        let repo_path = repo_parent.path().join("workspace");

        let workspace = create_workspace_request(&app, &repo_path, "repo").await;
        assert_eq!(workspace.kind, WorkspaceKind::Git);
        assert_eq!(workspace.root_path, repo_path.to_string_lossy());

        let refs_req = Request::builder()
            .method("GET")
            .uri(format!("/workspaces/{}/source-refs", workspace.id))
            .body(Body::empty())
            .expect("refs req");
        let refs_resp = app.clone().oneshot(refs_req).await.expect("refs resp");
        assert_eq!(refs_resp.status(), StatusCode::OK);
        let refs: Vec<SourceRef> = decode_json(
            refs_resp
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        );
        assert!(refs.iter().any(|item| item.name == "main" && item.kind == SourceRefKind::LocalBranch));
        assert!(refs.iter().any(|item| item.name == "local-only" && item.kind == SourceRefKind::LocalBranch));
        assert!(refs.iter().any(|item| item.name == "origin/main" && item.kind == SourceRefKind::RemoteTrackingBranch));

        let create_worktree_req = Request::builder()
            .method("POST")
            .uri(format!("/workspaces/{}/worktrees", workspace.id))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "source_ref": "origin/main",
                    "branch_name": "feature-remote",
                }))
                .expect("json"),
            ))
            .expect("worktree req");
        let create_worktree_resp = app
            .clone()
            .oneshot(create_worktree_req)
            .await
            .expect("worktree resp");
        assert_eq!(create_worktree_resp.status(), StatusCode::OK);
        let managed_worktree: ManagedWorktree = decode_json(
            create_worktree_resp
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        );
        assert_eq!(managed_worktree.branch_name, "feature-remote");
        assert_eq!(managed_worktree.source_ref, "origin/main");
        assert!(Path::new(&managed_worktree.path).exists());

        let duplicate_req = Request::builder()
            .method("POST")
            .uri(format!("/workspaces/{}/worktrees", workspace.id))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "source_ref": "main",
                    "branch_name": "feature-remote",
                }))
                .expect("json"),
            ))
            .expect("duplicate req");
        let duplicate_resp = app.clone().oneshot(duplicate_req).await.expect("duplicate resp");
        assert_eq!(duplicate_resp.status(), StatusCode::CONFLICT);

        let list_worktrees_req = Request::builder()
            .method("GET")
            .uri(format!("/workspaces/{}/worktrees", workspace.id))
            .body(Body::empty())
            .expect("list worktrees req");
        let list_worktrees_resp = app
            .clone()
            .oneshot(list_worktrees_req)
            .await
            .expect("list worktrees resp");
        assert_eq!(list_worktrees_resp.status(), StatusCode::OK);
        let worktrees: Vec<ManagedWorktree> = decode_json(
            list_worktrees_resp
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        );
        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].id, managed_worktree.id);
    }

    #[tokio::test]
    async fn validate_non_git_workspace_rejects_worktree_flows() {
        let temp = tempdir().expect("tempdir");
        let runtime: Arc<dyn SessionRuntime> = Arc::new(MockRuntime::default());
        let app = build_router(test_state(runtime, temp.path().join("control.sqlite")));

        let workspace = create_workspace_request(&app, temp.path(), "plain").await;
        assert_eq!(workspace.kind, WorkspaceKind::None);

        let refs_req = Request::builder()
            .method("GET")
            .uri(format!("/workspaces/{}/source-refs", workspace.id))
            .body(Body::empty())
            .expect("refs req");
        let refs_resp = app.clone().oneshot(refs_req).await.expect("refs resp");
        let refs: Vec<SourceRef> = decode_json(
            refs_resp
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        );
        assert!(refs.is_empty());

        let worktree_req = Request::builder()
            .method("POST")
            .uri(format!("/workspaces/{}/worktrees", workspace.id))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "source_ref": "main",
                    "branch_name": "feature-a",
                }))
                .expect("json"),
            ))
            .expect("worktree req");
        let worktree_resp = app.clone().oneshot(worktree_req).await.expect("worktree resp");
        assert_eq!(worktree_resp.status(), StatusCode::BAD_REQUEST);

        let session_req = Request::builder()
            .method("POST")
            .uri("/sessions")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "workspace_id": workspace.id,
                    "kind": "worktree",
                    "managed_worktree_id": "missing",
                }))
                .expect("json"),
            ))
            .expect("session req");
        let session_resp = app.clone().oneshot(session_req).await.expect("session resp");
        assert_eq!(session_resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn validate_session_runtime_cwd_and_restart_persistence() {
        let temp = tempdir().expect("tempdir");
        let runtime = Arc::new(MockRuntime::default());
        let runtime_trait: Arc<dyn SessionRuntime> = runtime.clone();
        let store_path = temp.path().join("control.sqlite");
        let app = build_router(test_state(runtime_trait.clone(), store_path.clone()));
        let (repo_parent, _remote_parent) = init_git_workspace();
        let repo_path = repo_parent.path().join("workspace");

        let workspace = create_workspace_request(&app, &repo_path, "repo").await;

        let worktree_req = Request::builder()
            .method("POST")
            .uri(format!("/workspaces/{}/worktrees", workspace.id))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "source_ref": "main",
                    "branch_name": "feature-local",
                }))
                .expect("json"),
            ))
            .expect("worktree req");
        let worktree_resp = app.clone().oneshot(worktree_req).await.expect("worktree resp");
        let worktree: ManagedWorktree = decode_json(
            worktree_resp
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        );

        let local_req = Request::builder()
            .method("POST")
            .uri("/sessions")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "name": "local-session",
                    "workspace_id": workspace.id,
                    "kind": "local",
                }))
                .expect("json"),
            ))
            .expect("local req");
        let local_resp = app.clone().oneshot(local_req).await.expect("local resp");
        let local_session: Session = decode_json(
            local_resp
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        );
        assert_eq!(local_session.kind, SessionKind::Local);

        let worktree_session_req = Request::builder()
            .method("POST")
            .uri("/sessions")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "name": "worktree-session",
                    "workspace_id": workspace.id,
                    "kind": "worktree",
                    "managed_worktree_id": worktree.id,
                }))
                .expect("json"),
            ))
            .expect("worktree session req");
        let worktree_session_resp = app
            .clone()
            .oneshot(worktree_session_req)
            .await
            .expect("worktree session resp");
        let created_worktree_session: Session = decode_json(
            worktree_session_resp
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        );
        assert_eq!(created_worktree_session.kind, SessionKind::Worktree);
        assert_eq!(
            created_worktree_session
                .managed_worktree
                .as_ref()
                .expect("managed worktree")
                .id,
            worktree.id
        );

        let calls = runtime.create_calls.lock().expect("lock").clone();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].cwd, repo_path);
        assert_eq!(calls[1].cwd, PathBuf::from(&worktree.path));

        let app_after_restart = build_router(test_state(runtime_trait, store_path));

        let workspaces_req = Request::builder()
            .method("GET")
            .uri("/workspaces")
            .body(Body::empty())
            .expect("workspaces req");
        let workspaces_resp = app_after_restart
            .clone()
            .oneshot(workspaces_req)
            .await
            .expect("workspaces resp");
        let workspaces: Vec<Workspace> = decode_json(
            workspaces_resp
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        );
        assert_eq!(workspaces.len(), 1);

        let sessions_req = Request::builder()
            .method("GET")
            .uri("/sessions")
            .body(Body::empty())
            .expect("sessions req");
        let sessions_resp = app_after_restart
            .clone()
            .oneshot(sessions_req)
            .await
            .expect("sessions resp");
        let sessions: Vec<Session> = decode_json(
            sessions_resp
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        );
        assert_eq!(sessions.len(), 2);
        assert!(sessions.iter().any(|session| {
            session.id == local_session.id
                && session.kind == SessionKind::Local
                && session.workspace.id == workspace.id
        }));
        assert!(sessions.iter().any(|session| {
            session.id == created_worktree_session.id
                && session.kind == SessionKind::Worktree
                && session
                    .managed_worktree
                    .as_ref()
                    .is_some_and(|item| item.id == worktree.id)
        }));
    }

    #[tokio::test]
    async fn validate_websocket_lifecycle_delivery() {
        let temp = tempdir().expect("tempdir");
        let runtime: Arc<dyn SessionRuntime> = Arc::new(MockRuntime::default());
        let app = build_router(test_state(runtime, temp.path().join("control.sqlite")));
        let workspace = create_workspace_request(&app, temp.path(), "sandbox").await;
        let (addr, server_handle) = spawn_server(app).await;

        let ws_url = format!("ws://{addr}/ws/events");
        let (mut ws, _) = connect_async(ws_url).await.expect("connect ws");

        let client = Client::new();
        let base = format!("http://{addr}");
        let create_resp = client
            .post(format!("{base}/sessions"))
            .json(&serde_json::json!({
                "name": "beta",
                "workspace_id": workspace.id,
                "kind": "local",
            }))
            .send()
            .await
            .expect("create")
            .json::<Session>()
            .await
            .expect("json");

        let created_msg = ws
            .next()
            .await
            .expect("created next")
            .expect("created msg")
            .into_text()
            .expect("text");
        let created_event: LifecycleEvent = serde_json::from_str(&created_msg).expect("event json");
        assert_eq!(created_event.event_type, "session.created");
        assert_eq!(created_event.session_id, create_resp.id);

        let status = client
            .delete(format!("{base}/sessions/{}", create_resp.id))
            .send()
            .await
            .expect("terminate")
            .status();
        assert_eq!(status, StatusCode::NO_CONTENT);

        let term_msg = ws
            .next()
            .await
            .expect("terminated next")
            .expect("terminated msg")
            .into_text()
            .expect("text");
        let term_event: LifecycleEvent = serde_json::from_str(&term_msg).expect("event json");
        assert_eq!(term_event.event_type, "session.terminated");
        assert_eq!(term_event.session_id, create_resp.id);

        server_handle.abort();
    }

    #[tokio::test]
    async fn validate_terminal_routes_are_feature_gated() {
        let temp = tempdir().expect("tempdir");
        let runtime: Arc<dyn SessionRuntime> = Arc::new(MockRuntime::default());
        let app = build_router(test_state(runtime, temp.path().join("control.sqlite")));
        let workspace = create_workspace_request(&app, temp.path(), "sandbox").await;

        let create_req = Request::builder()
            .method("POST")
            .uri("/sessions")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "name": "alpha",
                    "workspace_id": workspace.id,
                    "kind": "local",
                }))
                .expect("json"),
            ))
            .expect("create req");
        let create_resp = app.clone().oneshot(create_req).await.expect("create resp");
        let created: Session = decode_json(
            create_resp
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        );

        let terminal_req = Request::builder()
            .method("GET")
            .uri(format!("/sessions/{}/terminal", created.id))
            .body(Body::empty())
            .expect("terminal req");
        let terminal_resp = app.oneshot(terminal_req).await.expect("terminal resp");
        assert_eq!(terminal_resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn validate_shell_routes_and_assets_are_served_without_api_regressions() {
        let temp = tempdir().expect("tempdir");
        let runtime: Arc<dyn SessionRuntime> = Arc::new(MockRuntime::default());
        let app = build_router(test_state(runtime, temp.path().join("control.sqlite")));

        let app_req = Request::builder()
            .method("GET")
            .uri("/app")
            .body(Body::empty())
            .expect("app req");
        let app_resp = app.clone().oneshot(app_req).await.expect("app resp");
        assert_eq!(app_resp.status(), StatusCode::OK);
        let app_html = String::from_utf8(
            app_resp
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes()
                .to_vec(),
        )
        .expect("html");
        assert!(app_html.contains(r#"<div id="shell-root"></div>"#));
        assert!(app_html.contains(r#"/app/assets/app.js"#));

        let session_app_req = Request::builder()
            .method("GET")
            .uri("/app/sessions/demo-session")
            .body(Body::empty())
            .expect("session app req");
        let session_app_resp = app
            .clone()
            .oneshot(session_app_req)
            .await
            .expect("session app resp");
        assert_eq!(session_app_resp.status(), StatusCode::OK);

        let asset_req = Request::builder()
            .method("GET")
            .uri("/app/assets/app.css")
            .body(Body::empty())
            .expect("asset req");
        let asset_resp = app.clone().oneshot(asset_req).await.expect("asset resp");
        assert_eq!(asset_resp.status(), StatusCode::OK);
        assert_eq!(
            asset_resp
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("text/css; charset=utf-8")
        );

        let api_req = Request::builder()
            .method("GET")
            .uri("/sessions")
            .body(Body::empty())
            .expect("api req");
        let api_resp = app.oneshot(api_req).await.expect("api resp");
        assert_eq!(api_resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn validate_terminal_surface_and_input_contract() {
        let temp = tempdir().expect("tempdir");
        let runtime = Arc::new(MockRuntime::default());
        let runtime_trait: Arc<dyn SessionRuntime> = runtime.clone();
        let app = build_router(terminal_enabled_state(
            runtime_trait,
            temp.path().join("control.sqlite"),
        ));
        let workspace = create_workspace_request(&app, temp.path(), "sandbox").await;

        let create_req = Request::builder()
            .method("POST")
            .uri("/sessions")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "name": "alpha",
                    "workspace_id": workspace.id,
                    "kind": "local",
                }))
                .expect("json"),
            ))
            .expect("create req");
        let create_resp = app.clone().oneshot(create_req).await.expect("create resp");
        let created: Session = decode_json(
            create_resp
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        );

        let terminal_req = Request::builder()
            .method("GET")
            .uri(format!("/sessions/{}/terminal", created.id))
            .body(Body::empty())
            .expect("terminal req");
        let terminal_resp = app
            .clone()
            .oneshot(terminal_req)
            .await
            .expect("terminal resp");
        assert_eq!(terminal_resp.status(), StatusCode::OK);
        let surface: TerminalSurfaceState = decode_json(
            terminal_resp
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        );

        assert_eq!(surface.session_id, created.id);
        assert_eq!(surface.stack.escape_parser, "vte");
        assert_eq!(surface.stack.state_core, terminal::TerminalStateCore::Vt100);
        assert_eq!(
            surface.fallback_policy.alternate_state_core,
            terminal::TerminalStateCore::AlacrittyTerminal
        );
        assert_eq!(
            surface.input_capabilities.named_keys,
            vec![
                TerminalNamedKey::Ctrl,
                TerminalNamedKey::Escape,
                TerminalNamedKey::Tab,
                TerminalNamedKey::ArrowUp,
                TerminalNamedKey::ArrowDown,
                TerminalNamedKey::ArrowLeft,
                TerminalNamedKey::ArrowRight,
                TerminalNamedKey::Enter,
            ]
        );
        let serialized = serde_json::to_value(&surface).expect("serialize");
        assert!(serialized.get("runtime_name").is_none());
        assert!(serialized.get("pane_id").is_none());

        let input_req = Request::builder()
            .method("POST")
            .uri(format!("/sessions/{}/terminal/input", created.id))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&TerminalInputRequest {
                    events: vec![
                        TerminalInputEvent::Text {
                            text: "ls".to_string(),
                        },
                        TerminalInputEvent::Key {
                            key: TerminalKey::Named {
                                key: TerminalNamedKey::Enter,
                            },
                            ctrl: false,
                            alt: false,
                            shift: false,
                        },
                    ],
                })
                .expect("json"),
            ))
            .expect("input req");
        let input_resp = app.oneshot(input_req).await.expect("input resp");
        assert_eq!(input_resp.status(), StatusCode::OK);
        let input_result: TerminalInputResponse = decode_json(
            input_resp
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        );
        assert_eq!(input_result.accepted_events, 2);

        let recorded_inputs = runtime.inputs.lock().expect("lock");
        assert_eq!(recorded_inputs.len(), 1);
        assert_eq!(recorded_inputs[0].events.len(), 2);
        assert!(matches!(
            recorded_inputs[0].events[1],
            TerminalInputEvent::Key {
                key: TerminalKey::Named {
                    key: TerminalNamedKey::Enter
                },
                ctrl: false,
                alt: false,
                shift: false,
            }
        ));
    }

    #[tokio::test]
    async fn validate_terminal_snapshot_bootstrap_includes_scrollback() {
        let temp = tempdir().expect("tempdir");
        let runtime = Arc::new(MockRuntime::default());
        let runtime_trait: Arc<dyn SessionRuntime> = runtime.clone();
        let app = build_router(terminal_enabled_state(
            runtime_trait,
            temp.path().join("control.sqlite"),
        ));
        let workspace = create_workspace_request(&app, temp.path(), "sandbox").await;
        let created = create_local_session_request(&app, &workspace.id, "alpha").await;

        runtime
            .snapshots
            .lock()
            .expect("lock")
            .insert("alpha".to_string(), sample_snapshot_with_scrollback());

        let terminal_req = Request::builder()
            .method("GET")
            .uri(format!("/sessions/{}/terminal", created.id))
            .body(Body::empty())
            .expect("terminal req");
        let terminal_resp = app.oneshot(terminal_req).await.expect("terminal resp");
        assert_eq!(terminal_resp.status(), StatusCode::OK);
        let surface: TerminalSurfaceState = decode_json(
            terminal_resp
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes(),
        );

        assert_eq!(surface.snapshot.scrollback.len(), 1);
        assert_eq!(surface.snapshot.scrollback[0].cells[0].text, "$");
        assert!(surface.snapshot.plain_text.starts_with("$\n"));
    }

    #[tokio::test]
    async fn validate_unknown_terminal_stream_session_is_rejected() {
        let temp = tempdir().expect("tempdir");
        let runtime: Arc<dyn SessionRuntime> = Arc::new(MockRuntime::default());
        let app = build_router(terminal_enabled_state(runtime, temp.path().join("control.sqlite")));
        let (addr, server_handle) = spawn_server(app).await;

        let ws_url = format!("ws://{addr}/sessions/missing/terminal/stream");
        let error = connect_async(ws_url).await.expect_err("missing stream should fail");
        match error {
            tokio_tungstenite::tungstenite::Error::Http(response) => {
                assert_eq!(response.status(), StatusCode::NOT_FOUND);
            }
            other => panic!("unexpected websocket error: {other:?}"),
        }

        server_handle.abort();
    }

    #[tokio::test]
    async fn validate_terminal_stream_sequences_are_monotonic() {
        let temp = tempdir().expect("tempdir");
        let runtime = Arc::new(MockRuntime::default());
        let runtime_trait: Arc<dyn SessionRuntime> = runtime.clone();
        let app = build_router(terminal_enabled_state(
            runtime_trait,
            temp.path().join("control.sqlite"),
        ));
        let workspace = create_workspace_request(&app, temp.path(), "sandbox").await;
        let created = create_local_session_request(&app, &workspace.id, "alpha").await;
        let (addr, server_handle) = spawn_server(app).await;

        let ws_url = format!("ws://{addr}/sessions/{}/terminal/stream", created.id);
        let (mut ws, _) = connect_async(ws_url).await.expect("connect terminal ws");

        let initial = read_terminal_frame(&mut ws).await;
        assert_eq!(initial.sequence, 1);
        assert_eq!(initial.session_id, created.id);

        runtime
            .snapshots
            .lock()
            .expect("lock")
            .insert("alpha".to_string(), snapshot_with_text("cde", "uvw"));
        let second = read_terminal_frame(&mut ws).await;
        assert_eq!(second.sequence, 2);

        runtime
            .snapshots
            .lock()
            .expect("lock")
            .insert("alpha".to_string(), snapshot_with_text("xyz", "uvw"));
        let third = read_terminal_frame(&mut ws).await;
        assert_eq!(third.sequence, 3);
        assert!(third.sequence > second.sequence);

        server_handle.abort();
    }

    #[tokio::test]
    async fn validate_terminal_stream_backpressure_coalesces_to_latest_snapshot() {
        let initial = sample_snapshot();
        let newer = snapshot_with_text("cde", "uvw");
        let latest = snapshot_with_text("xyz", "uvw");
        let (updates_tx, mut updates_rx) = watch::channel(TerminalStreamUpdate::Snapshot(initial.clone()));

        updates_tx
            .send(TerminalStreamUpdate::Snapshot(newer))
            .expect("send newer");
        updates_tx
            .send(TerminalStreamUpdate::Snapshot(latest.clone()))
            .expect("send latest");

        updates_rx.changed().await.expect("watch changed");
        let update = updates_rx.borrow().clone();
        let snapshot = match update {
            TerminalStreamUpdate::Snapshot(snapshot) => snapshot,
            TerminalStreamUpdate::Closed => panic!("stream unexpectedly closed"),
        };
        let frame = snapshot.diff_frame("session-1", 2, Some(&initial));

        assert_eq!(frame.sequence, 2);
        assert_eq!(frame.lines[0].cells[0].text, "x");
        assert_eq!(frame.lines[0].cells[1].text, "y");
        assert_eq!(frame.lines[0].cells[2].text, "z");
    }

    #[tokio::test]
    async fn validate_terminal_stream_closes_when_selected_session_terminates() {
        let temp = tempdir().expect("tempdir");
        let runtime = Arc::new(MockRuntime::default());
        let runtime_trait: Arc<dyn SessionRuntime> = runtime.clone();
        let app = build_router(terminal_enabled_state(
            runtime_trait,
            temp.path().join("control.sqlite"),
        ));
        let workspace = create_workspace_request(&app, temp.path(), "sandbox").await;
        let created = create_local_session_request(&app, &workspace.id, "alpha").await;
        let (addr, server_handle) = spawn_server(app).await;

        let ws_url = format!("ws://{addr}/sessions/{}/terminal/stream", created.id);
        let (mut ws, _) = connect_async(ws_url).await.expect("connect terminal ws");
        let _ = read_terminal_frame(&mut ws).await;

        let client = Client::new();
        let status = client
            .delete(format!("http://{addr}/sessions/{}", created.id))
            .send()
            .await
            .expect("terminate")
            .status();
        assert_eq!(status, StatusCode::NO_CONTENT);

        let closed = tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .expect("terminal close frame")
            .expect("close message")
            .expect("websocket result");
        assert!(matches!(closed, tokio_tungstenite::tungstenite::Message::Close(_)));

        server_handle.abort();
    }

    async fn read_terminal_frame(
        ws: &mut tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    ) -> TerminalStreamFrame {
        let message = tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .expect("terminal frame")
            .expect("frame next")
            .expect("frame result")
            .into_text()
            .expect("frame text");
        serde_json::from_str(&message).expect("terminal frame json")
    }
}
