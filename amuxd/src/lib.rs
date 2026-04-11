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
use terminal::{
    TerminalCore, TerminalCursor, TerminalInputEvent, TerminalInputRequest, TerminalInputResponse,
    TerminalSnapshot, TerminalSurfaceState,
};
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    config: AppConfig,
    runtime: Arc<dyn SessionRuntime>,
    store: Arc<RwLock<SessionStore>>,
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
        let store = SessionStore::load(store_path)?;
        Ok(Self {
            config,
            runtime,
            store: Arc::new(RwLock::new(store)),
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
    BadRequest(String),
    NotFound(String),
    Runtime(String),
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(msg) => write!(f, "{msg}"),
            Self::NotFound(msg) => write!(f, "{msg}"),
            Self::Runtime(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::BadRequest(message) => (
                StatusCode::BAD_REQUEST,
                Json(ErrorEnvelope {
                    error: ErrorBody {
                        code: "invalid_request".to_string(),
                        message,
                    },
                }),
            )
                .into_response(),
            Self::NotFound(message) => (
                StatusCode::NOT_FOUND,
                Json(ErrorEnvelope {
                    error: ErrorBody {
                        code: "session_not_found".to_string(),
                        message,
                    },
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
    fn create(&self, name: Option<&str>) -> Result<RuntimeSession, AppError>;
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
    fn create(&self, name: Option<&str>) -> Result<RuntimeSession, AppError> {
        let base_name = name
            .filter(|v| !v.trim().is_empty())
            .map(sanitize_runtime_name)
            .unwrap_or_else(|| "session".to_string());
        let runtime_name = format!(
            "{}-{}",
            base_name,
            &Uuid::new_v4().simple().to_string()[..8]
        );

        let output = Command::new("tmux")
            .args(["new-session", "-d", "-s", &runtime_name])
            .output()
            .map_err(|e| AppError::Runtime(format!("failed to execute tmux new-session: {e}")))?;

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
            .map_err(|e| AppError::Runtime(format!("failed to execute tmux list-sessions: {e}")))?;

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
            .map_err(|e| AppError::Runtime(format!("failed to execute tmux kill-session: {e}")))?;

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
            .args(["capture-pane", "-e", "-p", "-t", runtime_name])
            .output()
            .map_err(|e| AppError::Runtime(format!("failed to execute tmux capture-pane: {e}")))?;

        if !output.status.success() {
            return Err(AppError::Runtime(format!(
                "tmux capture-pane failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        let mut core = TerminalCore::new(rows, cols, 0);
        core.ingest(&output.stdout);
        let mut snapshot = core.snapshot();
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
                            .map_err(|e| {
                                AppError::Runtime(format!("failed to execute tmux send-keys: {e}"))
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
                        .map_err(|e| {
                            AppError::Runtime(format!("failed to execute tmux resize-window: {e}"))
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
        .map_err(|e| AppError::Runtime(format!("failed to execute tmux send-keys: {e}")))?;
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
        .map_err(|e| AppError::Runtime(format!("failed to execute tmux display-message: {e}")))?;
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
        .map_err(|e| AppError::Runtime(format!("failed to execute tmux display-message: {e}")))?;
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
                    terminal::TerminalNamedKey::Ctrl => Err(AppError::BadRequest(
                        "bare ctrl key events are not supported; send a ctrl character chord"
                            .to_string(),
                    )),
                };
            }

            let mapped = match key {
                terminal::TerminalNamedKey::Ctrl => {
                    return Err(AppError::BadRequest(
                        "bare ctrl key events are not supported; send a ctrl character chord"
                            .to_string(),
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
                AppError::BadRequest("character key events require one character".to_string())
            })?;
            if chars.next().is_some() {
                return Err(AppError::BadRequest(
                    "character key events require exactly one character".to_string(),
                ));
            }

            if ctrl {
                if !ch.is_ascii() {
                    return Err(AppError::BadRequest(
                        "ctrl character chords currently require ASCII input".to_string(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSession {
    id: String,
    name: String,
    runtime_name: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct StoredSessionMap {
    sessions: HashMap<String, StoredSession>,
}

pub struct SessionStore {
    path: PathBuf,
    sessions: HashMap<String, StoredSession>,
}

impl SessionStore {
    fn load(path: PathBuf) -> Result<Self, AppError> {
        if !path.exists() {
            return Ok(Self {
                path,
                sessions: HashMap::new(),
            });
        }

        let raw = fs::read_to_string(&path)
            .map_err(|e| AppError::Runtime(format!("failed reading store file: {e}")))?;
        let persisted: StoredSessionMap = serde_json::from_str(&raw)
            .map_err(|e| AppError::Runtime(format!("failed parsing store file: {e}")))?;

        Ok(Self {
            path,
            sessions: persisted.sessions,
        })
    }

    fn save(&self) -> Result<(), AppError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| AppError::Runtime(format!("failed creating store directory: {e}")))?;
        }

        let payload = serde_json::to_string_pretty(&StoredSessionMap {
            sessions: self.sessions.clone(),
        })
        .map_err(|e| AppError::Runtime(format!("failed serializing store data: {e}")))?;

        fs::write(&self.path, payload)
            .map_err(|e| AppError::Runtime(format!("failed writing store file: {e}")))?;
        Ok(())
    }

    fn insert(&mut self, session: StoredSession) -> Result<(), AppError> {
        self.sessions.insert(session.id.clone(), session);
        self.save()
    }

    fn remove(&mut self, session_id: &str) -> Result<Option<StoredSession>, AppError> {
        let removed = self.sessions.remove(session_id);
        self.save()?;
        Ok(removed)
    }

    fn get(&self, session_id: &str) -> Option<StoredSession> {
        self.sessions.get(session_id).cloned()
    }

    fn all(&self) -> Vec<StoredSession> {
        self.sessions.values().cloned().collect()
    }
}

pub fn build_router(state: AppState) -> Router {
    let mut app = Router::new()
        .route("/health", get(get_health))
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

async fn create_session(
    State(state): State<AppState>,
    Json(payload): Json<CreateSessionRequest>,
) -> AppResult<Json<Session>> {
    let runtime_session = state.runtime.create(payload.name.as_deref())?;
    let id = Uuid::new_v4().to_string();
    let name = payload
        .name
        .unwrap_or_else(|| runtime_session.runtime_name.clone());

    let mut store = state.store.write().await;
    store.insert(StoredSession {
        id: id.clone(),
        name: name.clone(),
        runtime_name: runtime_session.runtime_name,
    })?;

    let response = Session {
        id: id.clone(),
        name,
        state: "running".to_string(),
        created_at: to_rfc3339_utc(runtime_session.created_at),
        last_activity_at: to_rfc3339_utc(runtime_session.last_activity_at),
    };

    let _ = state.events_tx.send(LifecycleEvent {
        event_id: Uuid::new_v4().to_string(),
        event_type: "session.created".to_string(),
        occurred_at: now_rfc3339(),
        session_id: id,
    });

    Ok(Json(response))
}

async fn list_sessions(State(state): State<AppState>) -> AppResult<Json<Vec<Session>>> {
    let runtime_sessions = state.runtime.list()?;
    let runtime_index: HashMap<String, RuntimeSession> = runtime_sessions
        .into_iter()
        .map(|s| (s.runtime_name.clone(), s))
        .collect();

    let store = state.store.read().await;
    let mut sessions: Vec<_> = store
        .all()
        .into_iter()
        .filter_map(|stored| {
            runtime_index.get(&stored.runtime_name).map(|runtime| {
                (
                    runtime.created_at,
                    Session {
                        id: stored.id,
                        name: stored.name,
                        state: "running".to_string(),
                        created_at: to_rfc3339_utc(runtime.created_at),
                        last_activity_at: to_rfc3339_utc(runtime.last_activity_at),
                    },
                )
            })
        })
        .collect();
    sessions.sort_by(|left, right| right.0.cmp(&left.0));

    Ok(Json(
        sessions.into_iter().map(|(_, session)| session).collect(),
    ))
}

async fn get_session(
    State(state): State<AppState>,
    AxumPath(session_id): AxumPath<String>,
) -> AppResult<Json<Session>> {
    let stored = {
        let store = state.store.read().await;
        store
            .get(&session_id)
            .ok_or_else(|| AppError::NotFound(format!("session not found: {session_id}")))?
    };

    let runtime_sessions = state.runtime.list()?;
    let runtime = runtime_sessions
        .into_iter()
        .find(|s| s.runtime_name == stored.runtime_name)
        .ok_or_else(|| AppError::NotFound(format!("session not found: {session_id}")))?;

    Ok(Json(Session {
        id: stored.id,
        name: stored.name,
        state: "running".to_string(),
        created_at: to_rfc3339_utc(runtime.created_at),
        last_activity_at: to_rfc3339_utc(runtime.last_activity_at),
    }))
}

async fn terminate_session(
    State(state): State<AppState>,
    AxumPath(session_id): AxumPath<String>,
) -> AppResult<StatusCode> {
    let stored = {
        let store = state.store.read().await;
        store
            .get(&session_id)
            .ok_or_else(|| AppError::NotFound(format!("session not found: {session_id}")))?
    };

    let runtime_sessions = state.runtime.list()?;
    if runtime_sessions
        .iter()
        .all(|session| session.runtime_name != stored.runtime_name)
    {
        let mut store = state.store.write().await;
        let _ = store.remove(&session_id)?;
        return Err(AppError::NotFound(format!(
            "session not found: {session_id}"
        )));
    }

    state.runtime.terminate(&stored.runtime_name)?;
    let mut store = state.store.write().await;
    let _ = store.remove(&session_id)?;

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
    let stored = stored_session_by_id(&state, &session_id).await?;
    let snapshot = state.runtime.capture_terminal(&stored.runtime_name)?;
    Ok(Json(TerminalSurfaceState::baseline(session_id, snapshot)))
}

async fn post_terminal_input(
    State(state): State<AppState>,
    AxumPath(session_id): AxumPath<String>,
    Json(payload): Json<TerminalInputRequest>,
) -> AppResult<Json<TerminalInputResponse>> {
    let stored = stored_session_by_id(&state, &session_id).await?;
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

async fn stored_session_by_id(
    state: &AppState,
    session_id: &str,
) -> Result<StoredSession, AppError> {
    let store = state.store.read().await;
    store
        .get(session_id)
        .ok_or_else(|| AppError::NotFound(format!("session not found: {session_id}")))
}

pub fn default_store_path(data_dir: &Path) -> PathBuf {
    data_dir.join("sessions.json")
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
    use tempfile::tempdir;
    use terminal::{
        EscapeSequenceMetrics, TerminalCell, TerminalColor, TerminalKey, TerminalLine,
        TerminalModes, TerminalNamedKey,
    };
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;
    use tokio_tungstenite::connect_async;
    use tower::ServiceExt;

    #[derive(Default)]
    struct MockRuntime {
        sessions: Mutex<HashMap<String, RuntimeSession>>,
        snapshots: Mutex<HashMap<String, TerminalSnapshot>>,
        inputs: Mutex<Vec<TerminalInputRequest>>,
    }

    impl SessionRuntime for MockRuntime {
        fn create(&self, name: Option<&str>) -> Result<RuntimeSession, AppError> {
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
                .insert(runtime_name, session.clone());
            self.snapshots
                .lock()
                .expect("lock")
                .insert(session.runtime_name.clone(), sample_snapshot());
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
                .ok_or_else(|| AppError::NotFound("session not found".to_string()))?;
            self.snapshots.lock().expect("lock").remove(runtime_name);
            Ok(())
        }

        fn capture_terminal(&self, runtime_name: &str) -> Result<TerminalSnapshot, AppError> {
            self.snapshots
                .lock()
                .expect("lock")
                .get(runtime_name)
                .cloned()
                .ok_or_else(|| AppError::NotFound("session not found".to_string()))
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
                return Err(AppError::NotFound("session not found".to_string()));
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
            plain_text: "ab\n😀".to_string(),
        }
    }

    #[tokio::test]
    async fn validate_rest_lifecycle_flow() {
        let temp = tempdir().expect("tempdir");
        let runtime: Arc<dyn SessionRuntime> = Arc::new(MockRuntime::default());
        let state = test_state(runtime, temp.path().join("sessions.json"));
        let app = build_router(state.clone());

        let create_req = Request::builder()
            .method("POST")
            .uri("/sessions")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"alpha"}"#))
            .expect("create req");
        let create_resp = app.clone().oneshot(create_req).await.expect("create resp");
        assert_eq!(create_resp.status(), StatusCode::OK);
        let create_body = create_resp
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let created: Session = serde_json::from_slice(&create_body).expect("session json");

        let list_req = Request::builder()
            .method("GET")
            .uri("/sessions")
            .body(Body::empty())
            .expect("list req");
        let list_resp = app.clone().oneshot(list_req).await.expect("list resp");
        assert_eq!(list_resp.status(), StatusCode::OK);

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

    #[tokio::test]
    async fn validate_websocket_lifecycle_delivery() {
        let temp = tempdir().expect("tempdir");
        let runtime: Arc<dyn SessionRuntime> = Arc::new(MockRuntime::default());
        let state = test_state(runtime, temp.path().join("sessions.json"));
        let app = build_router(state);
        let (addr, server_handle) = spawn_server(app).await;

        let ws_url = format!("ws://{addr}/ws/events");
        let (mut ws, _) = connect_async(ws_url).await.expect("connect ws");

        let client = Client::new();
        let base = format!("http://{addr}");
        let create_resp = client
            .post(format!("{base}/sessions"))
            .json(&serde_json::json!({"name":"beta"}))
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
    async fn validate_restart_visibility() {
        let temp = tempdir().expect("tempdir");
        let store_path = temp.path().join("sessions.json");
        let runtime: Arc<dyn SessionRuntime> = Arc::new(MockRuntime::default());

        let state1 = test_state(runtime.clone(), store_path.clone());
        let app1 = build_router(state1);
        let create_req = Request::builder()
            .method("POST")
            .uri("/sessions")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"gamma"}"#))
            .expect("create req");
        let create_resp = app1.oneshot(create_req).await.expect("create resp");
        let body = create_resp
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let created: Session = serde_json::from_slice(&body).expect("session json");

        let state2 = test_state(runtime, store_path);
        let app2 = build_router(state2);
        let list_req = Request::builder()
            .method("GET")
            .uri("/sessions")
            .body(Body::empty())
            .expect("list req");
        let list_resp = app2.oneshot(list_req).await.expect("list resp");
        let list_body = list_resp
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let sessions: Vec<Session> = serde_json::from_slice(&list_body).expect("sessions json");

        assert!(sessions.iter().any(|s| s.id == created.id));
    }

    #[tokio::test]
    async fn validate_session_list_is_sorted_descending_by_created_at() {
        let temp = tempdir().expect("tempdir");
        let runtime: Arc<dyn SessionRuntime> = Arc::new(MockRuntime::default());
        let state = test_state(runtime, temp.path().join("sessions.json"));
        let app = build_router(state);

        let first = Request::builder()
            .method("POST")
            .uri("/sessions")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"alpha"}"#))
            .expect("first req");
        let first_resp = app.clone().oneshot(first).await.expect("first resp");
        let first_body = first_resp
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let first_session: Session = serde_json::from_slice(&first_body).expect("session json");

        tokio::time::sleep(std::time::Duration::from_millis(5)).await;

        let second = Request::builder()
            .method("POST")
            .uri("/sessions")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"beta"}"#))
            .expect("second req");
        let second_resp = app.clone().oneshot(second).await.expect("second resp");
        let second_body = second_resp
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let second_session: Session = serde_json::from_slice(&second_body).expect("session json");

        let list_req = Request::builder()
            .method("GET")
            .uri("/sessions")
            .body(Body::empty())
            .expect("list req");
        let list_resp = app.oneshot(list_req).await.expect("list resp");
        let list_body = list_resp
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let sessions: Vec<Session> = serde_json::from_slice(&list_body).expect("sessions json");

        assert_eq!(sessions[0].id, second_session.id);
        assert_eq!(sessions[1].id, first_session.id);
    }

    #[tokio::test]
    async fn validate_terminal_routes_are_feature_gated() {
        let temp = tempdir().expect("tempdir");
        let runtime: Arc<dyn SessionRuntime> = Arc::new(MockRuntime::default());
        let state = test_state(runtime, temp.path().join("sessions.json"));
        let app = build_router(state);

        let create_req = Request::builder()
            .method("POST")
            .uri("/sessions")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"alpha"}"#))
            .expect("create req");
        let create_resp = app.clone().oneshot(create_req).await.expect("create resp");
        let create_body = create_resp
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let created: Session = serde_json::from_slice(&create_body).expect("session json");

        let terminal_req = Request::builder()
            .method("GET")
            .uri(format!("/sessions/{}/terminal", created.id))
            .body(Body::empty())
            .expect("terminal req");
        let terminal_resp = app.oneshot(terminal_req).await.expect("terminal resp");
        assert_eq!(terminal_resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn validate_terminal_surface_and_input_contract() {
        let temp = tempdir().expect("tempdir");
        let runtime = Arc::new(MockRuntime::default());
        let runtime_trait: Arc<dyn SessionRuntime> = runtime.clone();
        let state = terminal_enabled_state(runtime_trait, temp.path().join("sessions.json"));
        let app = build_router(state);

        let create_req = Request::builder()
            .method("POST")
            .uri("/sessions")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"alpha"}"#))
            .expect("create req");
        let create_resp = app.clone().oneshot(create_req).await.expect("create resp");
        let create_body = create_resp
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let created: Session = serde_json::from_slice(&create_body).expect("session json");

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
        let terminal_body = terminal_resp
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let surface: TerminalSurfaceState =
            serde_json::from_slice(&terminal_body).expect("surface json");

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
        let input_body = input_resp
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let input_result: TerminalInputResponse =
            serde_json::from_slice(&input_body).expect("input json");
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
}
