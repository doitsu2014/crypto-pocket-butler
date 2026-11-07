//! Axum servers with graceful shutdown.

use tokio::net::TcpListener;

use crate::state::AppState;
use crate::worker::WorkerState;

/// Serves the API until SIGINT or SIGTERM.
///
/// # Errors
/// Returns the I/O error when the listener cannot serve.
pub async fn serve(listener: TcpListener, state: AppState) -> std::io::Result<()> {
    axum::serve(listener, crate::routes::router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
}

/// Serves the worker's listener until SIGINT or SIGTERM.
///
/// # Errors
/// Returns the I/O error when the listener cannot serve.
pub async fn serve_worker(listener: TcpListener, state: WorkerState) -> std::io::Result<()> {
    axum::serve(listener, crate::worker::router(state))
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
