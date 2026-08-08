//! Web UI: browser frontend for the agent server.
//!
//! Serves the SPA built by `nexus-web/` (Vite → `nexus-web/dist`) from the same
//! axum router that hosts `/ws`, so the browser has no cross-origin problem and
//! the server key can be injected straight into the HTML.
//!
//! Static assets are read from disk (`tower_http::services::ServeDir`); a later
//! phase can embed them with `rust-embed`. The built `dist/` is resolved in
//! order: `$NEXUS_WEB_DIR`, `<cwd>/nexus-web/dist`, `<exe dir>/nexus-web/dist`.
//! When none exists the router still starts and `/` answers with a hint page,
//! so a missing frontend never takes down the WebSocket server.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::Request,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use tower::service_fn;
use tower_http::services::ServeDir;

/// Environment variable pointing at a built frontend `dist/` directory.
const WEB_DIR_ENV: &str = "NEXUS_WEB_DIR";
/// Directory name of the frontend project (relative to cwd or exe dir).
const WEB_DIST_REL: &str = "nexus-web/dist";
/// Fallback used when the frontend has not been built yet.
const UNBUILT_PAGE: &str = r#"<!doctype html><html><head><meta charset="utf-8"><title>Nexus Web UI</title></head><body style="font-family: system-ui, sans-serif; max-width: 640px; margin: 4rem auto; line-height: 1.6"><h1>Nexus Web UI</h1><p>The web frontend is not built yet.</p><p>Build it with:</p><pre style="background:#f4f4f4; padding:1rem; border-radius:8px">cd nexus-web &amp;&amp; npm install &amp;&amp; npm run build</pre><p>or point <code>NEXUS_WEB_DIR</code> at a built <code>dist/</code> directory and restart the server.</p></body></html>"#;

/// Shared state for the web UI routes.
#[derive(Clone)]
struct WebUiState {
    /// Fully-rendered index.html with the server key + cwd injected.
    index_html: Option<Arc<String>>,
}

/// Build the web UI router. Missing frontend degrades to a hint page instead of
/// failing the whole server.
///
/// `cwd` is the agent process's working directory; it is injected into the page
/// so the browser can use it as the default `cwd` for `session/new`.
pub fn router(secret: String, cwd: String) -> Router {
    let dist = resolve_dist_dir();
    let state = WebUiState {
        index_html: dist.as_ref().map(|d| {
            Arc::new(inject_server_meta(&d.join("index.html"), &secret, &cwd))
        }),
    };

    let mut app = Router::new().route("/", get(spa));

    if let Some(dist) = dist {
        // Hashed assets are real files in dist/; every other path is a
        // client-side route and falls back to the *injected* index.html, so
        // deep links also see the server key + cwd meta tags.
        let injected = state.index_html.clone().expect("dist exists implies index_html");
        let spa_fallback = service_fn(move |_: Request<Body>| {
            let html = injected.clone();
            async move { Ok::<_, Infallible>(Html(html.as_ref().clone()).into_response()) }
        });
        let fallback = ServeDir::new(&dist).fallback(spa_fallback);
        app = app.fallback_service(fallback);
    }

    app.with_state(state)
}

/// SPA fallback: same injected index.html as `/`, so the server key + cwd are
/// present no matter which path the browser lands on.
async fn spa(State(state): State<WebUiState>) -> Response {
    match &state.index_html {
        Some(html) => Html(html.as_ref().clone()).into_response(),
        None => Html(UNBUILT_PAGE).into_response(),
    }
}

/// Locate the built frontend `dist/` directory, or `None` when not found.
fn resolve_dist_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var(WEB_DIR_ENV)
        && !dir.trim().is_empty()
    {
        let dir = PathBuf::from(dir);
        if dir.join("index.html").is_file() {
            return Some(dir);
        }
        tracing::warn!(dir = %dir.display(), "NEXUS_WEB_DIR set but has no index.html; ignoring");
    }

    let cwd_candidate = std::env::current_dir().ok()?.join(WEB_DIST_REL);
    if cwd_candidate.join("index.html").is_file() {
        return Some(cwd_candidate);
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let exe_candidate = exe_dir.join(WEB_DIST_REL);
            if exe_candidate.join("index.html").is_file() {
                return Some(exe_candidate);
            }
        }
    }

    tracing::info!(
        "web UI frontend not found (looked in $NEXUS_WEB_DIR, ./{WEB_DIST_REL}, <exe>/{WEB_DIST_REL}); \
         serving the WebSocket endpoint only"
    );
    None
}

/// Read the built index.html and inject the server key + cwd meta tags.
fn inject_server_meta(index_path: &Path, secret: &str, cwd: &str) -> String {
    match std::fs::read_to_string(index_path) {
        Ok(html) => inject_server_meta_html(&html, secret, cwd),
        Err(e) => {
            tracing::warn!(path = %index_path.display(), error = %e, "failed to read web UI index.html");
            UNBUILT_PAGE.to_string()
        }
    }
}

/// Inject `<meta name="nexus-server-key">` and `<meta name="nexus-server-cwd">`
/// into index.html so the browser can read the key (without it ever appearing
/// in the URL/history) and the default session cwd.
fn inject_server_meta_html(html: &str, secret: &str, cwd: &str) -> String {
    let metas = format!(
        "{key_meta}\n    {cwd_meta}",
        key_meta = format!(
            r#"<meta name="nexus-server-key" content="{}">"#,
            // Escape for HTML attribute context; the key is hex-ish but don't assume.
            html_escape(secret)
        ),
        cwd_meta = format!(
            r#"<meta name="nexus-server-cwd" content="{}">"#,
            html_escape(cwd)
        ),
    );
    // Insert before `</head>` when present, else prepend to the document.
    match html.find("</head>") {
        Some(idx) => format!("{}\n    {}\n  {}", &html[..idx], metas, &html[idx..]),
        None => format!("{metas}\n{html}"),
    }
}

/// Minimal HTML attribute escaping for the server key.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Startup banner line: the browser URL.
pub fn web_ui_url(bind_addr: SocketAddr) -> String {
    format!("http://{bind_addr}/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_quotes_and_angle_brackets() {
        let s = html_escape(r#"a&b"c<d>"#);
        assert_eq!(s, "a&amp;b&quot;c&lt;d&gt;");
    }

    #[test]
    fn inject_into_head_when_present() {
        let html = "<html><head><title>x</title></head><body></body></html>";
        let out = inject_server_meta_html(html, "secret-123", "C:\\work");
        assert!(out.contains(r#"<meta name="nexus-server-key" content="secret-123">"#));
        assert!(out.contains(r#"<meta name="nexus-server-cwd" content="C:\work">"#));
        assert!(out.find("<meta").unwrap() < out.find("</head>").unwrap());
    }

    #[test]
    fn prepend_when_no_head() {
        let html = "<html><body>hi</body></html>";
        let out = inject_server_meta_html(html, "k", "/");
        assert!(out.starts_with(r#"<meta name="nexus-server-key""#));
    }

    #[test]
    fn unbuilt_page_has_no_key() {
        // When the built index.html cannot be read, the fallback page must not
        // leak the server secret.
        let out = inject_server_meta(Path::new("definitely-missing-index.html"), "nope", "/");
        assert!(!out.contains("nope"));
    }
}
