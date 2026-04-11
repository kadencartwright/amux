use axum::extract::Path;
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use include_dir::{Dir, include_dir};

static SHELL_DIST: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../amuxshell-web/dist");

pub async fn shell_entry() -> Html<&'static str> {
    Html(shell_index())
}

pub async fn shell_session_entry(Path(_session_id): Path<String>) -> Html<&'static str> {
    Html(shell_index())
}

pub async fn shell_asset(Path(asset): Path<String>) -> Response {
    if asset.contains("..") {
        return StatusCode::NOT_FOUND.into_response();
    }

    let path = format!("assets/{asset}");
    let Some(file) = SHELL_DIST.get_file(path.as_str()) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let content_type = match asset.rsplit('.').next().unwrap_or_default() {
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "html" => "text/html; charset=utf-8",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    };

    ([(header::CONTENT_TYPE, content_type)], file.contents()).into_response()
}

fn shell_index() -> &'static str {
    SHELL_DIST
        .get_file("index.html")
        .and_then(|file| file.contents_utf8())
        .expect("shell entrypoint present")
}
