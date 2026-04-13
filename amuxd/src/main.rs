use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use amuxd::{AppConfig, AppState, TmuxRuntime, build_router, default_store_path};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let addr = env::var("AMUXD_ADDR")
        .ok()
        .and_then(|v| v.parse::<SocketAddr>().ok())
        .unwrap_or_else(|| "127.0.0.1:8080".parse().expect("valid default addr"));

    let data_dir = env::var("AMUXD_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./data"));
    let config = AppConfig {
        terminal_renderer_v1_enabled: env::var("AMUXD_TERMINAL_RENDERER_V1")
            .ok()
            .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "True")),
        terminal_http_input_migration_enabled: env::var("AMUXD_TERMINAL_HTTP_INPUT_MIGRATION")
            .ok()
            .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "True")),
    };

    let state =
        AppState::new_with_config(Arc::new(TmuxRuntime), default_store_path(&data_dir), config)
            .expect("failed to initialize app state");
    let app = build_router(state);

    let listener = TcpListener::bind(addr)
        .await
        .expect("failed to bind listener");

    println!("amuxd listening on http://{}", addr);
    axum::serve(listener, app)
        .await
        .expect("server exited unexpectedly");
}
