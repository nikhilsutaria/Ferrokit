use std::net::SocketAddr;

use ferrokit_server::{app, Config};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_env("FERROKIT_LOG").unwrap_or_else(|_| EnvFilter::new("info")))
        .compact()
        .init();

    let config = Config::from_env();
    let addr: SocketAddr = format!("{}:{}", config.bind, config.port).parse().expect("FERROKIT_BIND/FERROKIT_PORT");
    let state = app::State::new(config);
    match state.engine.qpdf_version() {
        Ok(v) => tracing::info!("PDF engine: {v}"),
        Err(e) => tracing::warn!("qpdf not available, server-side PDF tools will fail: {e}"),
    }

    state.addons.clone().watch(state.http.clone(), state.config.clone());

    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    tracing::info!("Ferrokit {} listening on http://{addr}", env!("CARGO_PKG_VERSION"));
    axum::serve(listener, app::router(state)).with_graceful_shutdown(shutdown()).await.expect("server");
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
