//! WebDAV server setup and lifecycle management.

use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use hyper::body::Incoming;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tracing;
use uuid::Uuid;

use filehub_auth::password::hasher::PasswordHasher;
use filehub_core::config::StorageWebDavConfig;
use filehub_core::error::AppError;
use filehub_core::types::id::StorageId;
use filehub_database::repositories::user::UserRepository;
use filehub_service::file::service::FileService;
use filehub_service::file::upload::UploadService;
use filehub_service::folder::service::FolderService;

use crate::handler::DavHandler;

/// WebDAV server configuration and state
#[derive(Debug)]
pub struct WebDavServer {
    /// DAV handler
    handler: Arc<DavHandler>,
    /// Server configuration
    config: StorageWebDavConfig,
}

impl WebDavServer {
    /// Create a new WebDAV server
    pub fn new(
        config: StorageWebDavConfig,
        file_service: Arc<FileService>,
        upload_service: Arc<UploadService>,
        folder_service: Arc<FolderService>,
        user_repo: Arc<UserRepository>,
        password_hasher: Arc<PasswordHasher>,
    ) -> Self {
        let handler = Arc::new(DavHandler::new(
            file_service,
            upload_service,
            folder_service,
            user_repo,
            password_hasher,
            config.auth_realm.clone(),
        ));

        Self { handler, config }
    }

    /// Start the WebDAV server
    pub async fn start(&self, mut cancel: watch::Receiver<bool>) -> Result<(), AppError> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.port));

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| AppError::internal(format!("Failed to bind WebDAV server: {}", e)))?;

        tracing::info!("WebDAV server listening on {}", addr);

        let handler = Arc::clone(&self.handler);

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, peer_addr)) => {
                            let handler = Arc::clone(&handler);
                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(handler, stream, peer_addr).await {
                                    tracing::error!("WebDAV connection error from {}: {}", peer_addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("WebDAV accept error: {}", e);
                        }
                    }
                }
                _ = cancel.changed() => {
                    if *cancel.borrow() {
                        tracing::info!("WebDAV server shutting down");
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle a single TCP connection using hyper
    async fn handle_connection(
        handler: Arc<DavHandler>,
        stream: tokio::net::TcpStream,
        peer_addr: SocketAddr,
    ) -> Result<(), AppError> {
        let io = hyper_util::rt::TokioIo::new(stream);

        let service = hyper::service::service_fn(move |req: Request<Incoming>| {
            let handler = Arc::clone(&handler);
            async move {
                let (parts, body) = req.into_parts();

                let body_bytes = match http_body_util::BodyExt::collect(body).await {
                    Ok(collected) => collected.to_bytes(),
                    Err(e) => {
                        tracing::error!("Failed to read request body: {}", e);
                        Bytes::new()
                    }
                };

                let req = Request::from_parts(parts, body_bytes.clone());

                let uri_path = req.uri().path().to_string();
                let (storage_id, resource_path) = parse_dav_path(&uri_path);

                let response = match storage_id {
                    Some(sid) => handler.handle(sid, &resource_path, req).await,
                    None => Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body("Invalid WebDAV path. Expected /dav/{storage_id}/...".to_string())
                        .unwrap_or_else(|_| {
                            let mut r = Response::new("Bad Request".to_string());
                            *r.status_mut() = StatusCode::BAD_REQUEST;
                            r
                        }),
                };

                let (parts, body_str) = response.into_parts();
                let body_full = http_body_util::Full::new(Bytes::from(body_str));
                Ok::<_, hyper::Error>(Response::from_parts(parts, body_full))
            }
        });

        let conn = hyper::server::conn::http1::Builder::new().serve_connection(io, service);

        if let Err(e) = conn.await {
            tracing::error!("WebDAV connection error from {}: {}", peer_addr, e);
        }

        Ok(())
    }
}

/// Parse `/dav/{storage_id}/path/to/resource` into (StorageId, path)
fn parse_dav_path(uri_path: &str) -> (Option<StorageId>, String) {
    let trimmed = uri_path.trim_start_matches('/');

    if !trimmed.starts_with("dav/") {
        return (None, uri_path.to_string());
    }

    let after_dav = &trimmed[4..];

    let (id_str, rest) = match after_dav.find('/') {
        Some(idx) => (&after_dav[..idx], &after_dav[idx..]),
        None => (after_dav, "/"),
    };

    match Uuid::parse_str(id_str) {
        Ok(uuid) => (Some(StorageId::from(uuid)), rest.to_string()),
        Err(_) => (None, uri_path.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dav_path_root() {
        let (sid, path) = parse_dav_path("/dav/550e8400-e29b-41d4-a716-446655440000/");
        assert!(sid.is_some());
        assert_eq!(path, "/");
    }

    #[test]
    fn test_parse_dav_path_with_resource() {
        let (sid, path) =
            parse_dav_path("/dav/550e8400-e29b-41d4-a716-446655440000/folder/file.txt");
        assert!(sid.is_some());
        assert_eq!(path, "/folder/file.txt");
    }

    #[test]
    fn test_parse_dav_path_invalid() {
        let (sid, _) = parse_dav_path("/api/files");
        assert!(sid.is_none());
    }

    #[test]
    fn test_parse_dav_path_no_slash_after_id() {
        let (sid, path) = parse_dav_path("/dav/550e8400-e29b-41d4-a716-446655440000");
        assert!(sid.is_some());
        assert_eq!(path, "/");
    }
}
