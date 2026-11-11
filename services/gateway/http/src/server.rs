//! Axum server with graceful shutdown.

use std::net::SocketAddr;

use tokio::net::TcpListener;

use crate::routes::router;
use crate::state::AppState;

/// Serves until SIGINT or SIGTERM. The peer address is attached to every request
/// so the edge can derive the trusted client IP (NFR23.3).
///
/// # Errors
/// Returns the I/O error when the listener cannot serve.
pub async fn serve(listener: TcpListener, state: AppState) -> std::io::Result<()> {
    axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
