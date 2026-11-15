//! A loopback-only file server for job directories, so Chromium loads uploaded pages and books
//! over http (relative links work, and the page can't reach the container's own files).

use std::collections::HashMap;
use std::hash::{BuildHasher, Hasher};
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::{Path as UrlPath, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;

/// Directories Chromium may read from, by unguessable token.
#[derive(Clone, Default)]
pub struct Jobs(Arc<Mutex<HashMap<String, PathBuf>>>);

/// Unregisters a job directory when dropped.
pub struct Guard {
    jobs: Jobs,
    token: String,
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Ok(mut m) = self.jobs.0.lock() {
            m.remove(&self.token);
        }
    }
}

/// 128 random bits (RandomState is seeded from the OS), as hex.
fn token() -> String {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let half = || {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(n);
        h.finish()
    };
    format!("{:016x}{:016x}", half(), half())
}

impl Jobs {
    pub fn add(&self, dir: &Path) -> (String, Guard) {
        let token = token();
        self.0.lock().unwrap().insert(token.clone(), dir.to_path_buf());
        (token.clone(), Guard { jobs: self.clone(), token })
    }
}

pub fn router(jobs: Jobs) -> Router {
    Router::new().route("/job/{token}/{*path}", get(file)).with_state(jobs)
}

async fn file(State(jobs): State<Jobs>, UrlPath((token, path)): UrlPath<(String, String)>) -> Response {
    let Some(dir) = jobs.0.lock().ok().and_then(|m| m.get(&token).cloned()) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let rel = Path::new(&path);
    if path.is_empty() || !rel.components().all(|c| matches!(c, Component::Normal(_))) {
        return StatusCode::NOT_FOUND.into_response();
    }
    match tokio::fs::read(dir.join(rel)).await {
        Ok(bytes) => {
            let mime = mime_guess::from_path(rel).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.essence_str().to_string())], bytes).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Listen on 127.0.0.1:8091 (or any free port) and return the address.
pub async fn start(jobs: Jobs) -> std::io::Result<SocketAddr> {
    let listener = match tokio::net::TcpListener::bind("127.0.0.1:8091").await {
        Ok(l) => l,
        Err(_) => tokio::net::TcpListener::bind("127.0.0.1:0").await?,
    };
    let addr = listener.local_addr()?;
    tokio::spawn(async move {
        let _ = axum::serve(listener, router(jobs)).await;
    });
    Ok(addr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn serves_registered_files_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "hi").unwrap();
        let jobs = Jobs::default();
        let (token, guard) = jobs.add(dir.path());
        let get = |path: String| Request::get(path).body(Body::empty()).unwrap();

        let ok = router(jobs.clone()).oneshot(get(format!("/job/{token}/index.html"))).await.unwrap();
        assert_eq!(ok.status(), StatusCode::OK);
        assert_eq!(ok.headers()["content-type"], "text/html");

        let up = router(jobs.clone()).oneshot(get(format!("/job/{token}/../index.html"))).await.unwrap();
        assert_eq!(up.status(), StatusCode::NOT_FOUND);

        drop(guard);
        let gone = router(jobs.clone()).oneshot(get(format!("/job/{token}/index.html"))).await.unwrap();
        assert_eq!(gone.status(), StatusCode::NOT_FOUND);
    }
}
