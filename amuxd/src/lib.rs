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
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    runtime: Arc<dyn SessionRuntime>,
    store: Arc<RwLock<SessionStore>>,
    events_tx: broadcast::Sender<LifecycleEvent>,
}

impl AppState {
    pub fn new(runtime: Arc<dyn SessionRuntime>, store_path: PathBuf) -> Result<Self, AppError> {
        let (events_tx, _) = broadcast::channel(1024);
        let store = SessionStore::load(store_path)?;
        Ok(Self {
            runtime,
            store: Arc::new(RwLock::new(store)),
            events_tx,
        })
    }
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
    NotFound(String),
    Runtime(String),
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "{msg}"),
            Self::Runtime(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
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
        let runtime_name = format!("{}-{}", base_name, &Uuid::new_v4().simple().to_string()[..8]);

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
    Router::new()
        .route("/health", get(get_health))
        .route("/sessions", post(create_session).get(list_sessions))
        .route(
            "/sessions/:session_id",
            get(get_session).delete(terminate_session),
        )
        .route("/ws/events", get(ws_events))
        .with_state(state)
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
    let sessions = store
        .all()
        .into_iter()
        .filter_map(|stored| {
            runtime_index.get(&stored.runtime_name).map(|runtime| Session {
                id: stored.id,
                name: stored.name,
                state: "running".to_string(),
                created_at: to_rfc3339_utc(runtime.created_at),
                last_activity_at: to_rfc3339_utc(runtime.last_activity_at),
            })
        })
        .collect();

    Ok(Json(sessions))
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
        return Err(AppError::NotFound(format!("session not found: {session_id}")));
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
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;
    use tokio_tungstenite::connect_async;
    use tower::ServiceExt;

    #[derive(Default)]
    struct MockRuntime {
        sessions: Mutex<HashMap<String, RuntimeSession>>,
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
            Ok(())
        }
    }

    fn test_state(runtime: Arc<dyn SessionRuntime>, store_path: PathBuf) -> AppState {
        AppState::new(runtime, store_path).expect("state")
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
        let create_body = create_resp.into_body().collect().await.expect("body").to_bytes();
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
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind listener");
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
        let body = create_resp.into_body().collect().await.expect("body").to_bytes();
        let created: Session = serde_json::from_slice(&body).expect("session json");

        let state2 = test_state(runtime, store_path);
        let app2 = build_router(state2);
        let list_req = Request::builder()
            .method("GET")
            .uri("/sessions")
            .body(Body::empty())
            .expect("list req");
        let list_resp = app2.oneshot(list_req).await.expect("list resp");
        let list_body = list_resp.into_body().collect().await.expect("body").to_bytes();
        let sessions: Vec<Session> = serde_json::from_slice(&list_body).expect("sessions json");

        assert!(sessions.iter().any(|s| s.id == created.id));
    }
}
