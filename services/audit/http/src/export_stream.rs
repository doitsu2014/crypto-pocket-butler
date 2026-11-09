//! Streams an export session into the response body as JSON Lines.
//!
//! Nothing is written to disk: the body *is* the export. The digest is taken over
//! the exact bytes sent, so the recorded `contentSha256` proves what the service
//! produced. A stream that cannot finish marks the export failed.

use std::io;

use audit_application::export::ExportSession;
use audit_application::hashing::StreamDigest;
use axum::body::Body;
use axum::http::{header, HeaderName, HeaderValue, StatusCode};
use axum::response::Response;
use bytes::Bytes;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::dto::EventDto;

/// The JSON Lines content type.
pub const NDJSON: &str = "application/x-ndjson";

/// Starts streaming `session` into a response body.
#[must_use]
pub fn stream_export(session: ExportSession) -> Response {
    let export_id = session.id().to_string();
    let (sender, receiver) = mpsc::channel::<Result<Bytes, io::Error>>(4);

    tokio::spawn(async move {
        let mut session = session;
        let mut digest = StreamDigest::default();
        loop {
            match session.next_batch().await {
                Ok(Some(batch)) => {
                    let mut chunk = Vec::new();
                    for event in &batch {
                        match serde_json::to_vec(&EventDto::from(event)) {
                            Ok(line) => {
                                chunk.extend_from_slice(&line);
                                chunk.push(b'\n');
                            }
                            Err(error) => {
                                let _ = session.fail(&error.to_string()).await;
                                return;
                            }
                        }
                    }
                    digest.update(&chunk);
                    if sender.send(Ok(Bytes::from(chunk))).await.is_err() {
                        let _ = session
                            .fail("the operator's connection closed before the export completed")
                            .await;
                        return;
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    let reason = error.to_string();
                    let _ = session.fail(&reason).await;
                    let _ = sender
                        .send(Err(io::Error::other(
                            "the export stream was cut short; the export is recorded as failed",
                        )))
                        .await;
                    return;
                }
            }
        }
        if let Err(error) = session.finish(&digest.finish()).await {
            tracing::error!(%error, "failed to record export completion");
        }
    });

    let mut response = Response::new(Body::from_stream(ReceiverStream::new(receiver)));
    *response.status_mut() = StatusCode::OK;
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(NDJSON));
    if let Ok(value) = HeaderValue::from_str(&export_id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-export-id"), value);
    }
    response
}
