use std::net::SocketAddr;

use axum::extract::DefaultBodyLimit;
use ferrokit_worker::caps::Caps;
use ferrokit_worker::{router, AppState};
use tracing_subscriber::EnvFilter;

fn var(k: &str) -> Option<String> {
    std::env::var(k).ok().filter(|v| !v.trim().is_empty())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_env("FERROKIT_LOG").unwrap_or_else(|_| EnvFilter::new("info")))
        .compact()
        .init();

    let port: u16 = var("FERROKIT_WORKER_PORT").and_then(|v| v.parse().ok()).unwrap_or(8090);
    let max_mb: usize = var("FERROKIT_MAX_UPLOAD_MB").and_then(|v| v.parse().ok()).unwrap_or(1024);

    let caps = Caps::detect();
    tracing::info!(office = ?caps.office_ops(), ocr = ?caps.ocr_ops(), languages = caps.languages.len(), "capabilities");
    let assets = std::path::PathBuf::from(var("FERROKIT_ASSETS").unwrap_or_else(|| "/usr/local/share/ferrokit".into()));
    let mut state = AppState::new(caps.clone()).with_assets(assets.clone());
    if let Some(exe) = caps.chromium.clone() {
        let jobs = ferrokit_worker::serve::Jobs::default();
        let addr = ferrokit_worker::serve::start(jobs.clone()).await.expect("loopback file server");
        let chrome = ferrokit_worker::chrome::Chrome::new(exe, format!("http://{addr}/"));
        state = state.with_printer(ferrokit_worker::print::Printer { chrome, jobs, base: format!("http://{addr}") });
    }
    if caps.unoserver {
        let n = var("FERROKIT_OFFICE_WORKERS").and_then(|v| v.parse().ok()).unwrap_or(2);
        let seed = Some(assets.join("registrymodifications.xcu")).filter(|p| p.exists());
        state = state.with_office(ferrokit_worker::office::Pool::start(n, seed).await);
    }

    let addr: SocketAddr = format!("0.0.0.0:{port}").parse().expect("FERROKIT_WORKER_PORT");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    tracing::info!("ferrokit-worker {} listening on {addr}", env!("CARGO_PKG_VERSION"));
    let app = router(state).layer(DefaultBodyLimit::max(max_mb * 1024 * 1024));
    axum::serve(listener, app).with_graceful_shutdown(shutdown()).await.expect("server");
}

async fn shutdown() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
    tracing::info!("shutting down");
}
