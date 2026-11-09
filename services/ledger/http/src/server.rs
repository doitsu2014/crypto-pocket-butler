//! Axum server with graceful shutdown.

use axum::Router;
use tokio::net::TcpListener;

/// Serves until SIGINT or SIGTERM, draining in-flight requests.
///
/// # Errors
/// Returns the I/O error when the listener cannot serve.
pub async fn serve(listener: TcpListener, router: Router) -> std::io::Result<()> {
    axum::serve(listener, router)
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
