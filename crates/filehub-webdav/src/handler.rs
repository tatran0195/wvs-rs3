//! WebDAV request handler — dispatches HTTP methods to implementations.

use std::sync::Arc;

use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use tracing;

use filehub_auth::password::hasher::PasswordHasher;
use filehub_core::error::AppError;
use filehub_core::types::id::StorageId;
use filehub_database::repositories::user::UserRepository;
use filehub_service::file::service::FileService;
use filehub_service::file::upload::UploadService;
use filehub_service::folder::service::FolderService;

use crate::auth::{AuthError, DavUser, extract_basic_credentials, unauthorized_response};
use crate::methods;
use crate::properties::Depth;

/// WebDAV request handler context
#[derive(Debug, Clone)]
pub struct DavHandler {
    /// File service
    file_service: Arc<FileService>,
    /// Upload service
    upload_service: Arc<UploadService>,
    /// Folder service
    folder_service: Arc<FolderService>,
    /// User repository for authentication
    user_repo: Arc<UserRepository>,
    /// Password hasher for verification
    password_hasher: Arc<PasswordHasher>,
    /// Auth realm for Basic auth
    auth_realm: String,
}

impl DavHandler {
    /// Create a new DAV handler
    pub fn new(
        file_service: Arc<FileService>,
        upload_service: Arc<UploadService>,
        folder_service: Arc<FolderService>,
        user_repo: Arc<UserRepository>,
        password_hasher: Arc<PasswordHasher>,
        auth_realm: String,
    ) -> Self {
        Self {
            file_service,
            upload_service,
            folder_service,
            user_repo,
            password_hasher,
            auth_realm,
        }
    }

    /// Handle a WebDAV request
    pub async fn handle(
        &self,
        storage_id: StorageId,
        path: &str,
        req: Request<Bytes>,
    ) -> Response<String> {
        let user = match self.authenticate(&req).await {
            Ok(u) => u,
            Err(_) => return unauthorized_response(&self.auth_realm),
        };

        let base_href = format!("/dav/{}", storage_id);
        let method = req.method().clone();
        let headers = req.headers().clone();
        let body_bytes = req.into_body();
        let body_str = String::from_utf8_lossy(&body_bytes).to_string();

        let result = match method.as_str() {
            "OPTIONS" => Ok(self.handle_options()),
            "PROPFIND" => {
                let depth = Depth::from_header(headers.get("Depth").and_then(|v| v.to_str().ok()));
                methods::handle_propfind(
                    &user,
                    storage_id,
                    path,
                    depth,
                    &body_str,
                    &self.folder_service,
                    &self.file_service,
                    &base_href,
                )
                .await
            }
            "PROPPATCH" => {
                methods::handle_proppatch(&user, storage_id, path, &body_str, &base_href).await
            }
            "MKCOL" => {
                methods::handle_mkcol(&user, storage_id, path, &body_str, &self.folder_service)
                    .await
            }
            "GET" => methods::handle_get(&user, storage_id, path, &self.file_service).await,
            "HEAD" => methods::handle_head(&user, storage_id, path, &self.file_service).await,
            "PUT" => {
                let content_type = headers
                    .get(http::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok());
                methods::handle_put(
                    &user,
                    storage_id,
                    path,
                    body_bytes,
                    content_type,
                    &self.file_service,
                    &self.upload_service,
                    &self.folder_service,
                )
                .await
            }
            "DELETE" => self.handle_delete(&user, storage_id, path).await,
            "COPY" => {
                methods::handle_copy(
                    &user,
                    storage_id,
                    path,
                    &headers,
                    &self.file_service,
                    &self.folder_service,
                    &base_href,
                )
                .await
            }
            "MOVE" => {
                methods::handle_move(
                    &user,
                    storage_id,
                    path,
                    &headers,
                    &self.file_service,
                    &self.folder_service,
                    &base_href,
                )
                .await
            }
            "LOCK" => {
                methods::handle_lock(&user, storage_id, path, &body_str, &self.file_service).await
            }
            "UNLOCK" => {
                let lock_token = headers.get("Lock-Token").and_then(|v| v.to_str().ok());
                methods::handle_unlock(&user, storage_id, path, lock_token, &self.file_service)
                    .await
            }
            _ => Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body(format!("Method {} not allowed", method))
                .unwrap_or_else(|_| {
                    let mut r = Response::new("Method not allowed".to_string());
                    *r.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
                    r
                })),
        };

        match result {
            Ok(response) => response,
            Err(e) => self.error_response(&e),
        }
    }

    /// Authenticate a request using Basic auth
    async fn authenticate(&self, req: &Request<Bytes>) -> Result<DavUser, AuthError> {
        let creds = extract_basic_credentials(req.headers())?;

        let user = self
            .user_repo
            .find_by_username(&creds.username)
            .await
            .map_err(|_| AuthError::AuthenticationFailed)?
            .ok_or(AuthError::AuthenticationFailed)?;

        if !matches!(
            user.status,
            filehub_entity::user::status::UserStatus::Active
        ) {
            return Err(AuthError::AccountLocked);
        }

        let valid = self
            .password_hasher
            .verify(&creds.password, &user.password_hash)
            .map_err(|_| AuthError::AuthenticationFailed)?;

        if !valid {
            return Err(AuthError::AuthenticationFailed);
        }

        Ok(DavUser::from_user(&user))
    }

    /// Handle OPTIONS request
    fn handle_options(&self) -> Response<String> {
        Response::builder()
            .status(StatusCode::OK)
            .header("Allow", "OPTIONS, PROPFIND, PROPPATCH, MKCOL, GET, HEAD, PUT, DELETE, COPY, MOVE, LOCK, UNLOCK")
            .header("DAV", "1, 2")
            .header("MS-Author-Via", "DAV")
            .body(String::new())
            .unwrap_or_else(|_| {
                let mut r = Response::new(String::new());
                *r.status_mut() = StatusCode::OK;
                r
            })
    }

    /// Handle DELETE request
    async fn handle_delete(
        &self,
        user: &DavUser,
        storage_id: StorageId,
        path: &str,
    ) -> Result<Response<String>, AppError> {
        tracing::debug!("DELETE: user={}, path='{}'", user.username, path);

        let trimmed = path.trim_matches('/');
        if trimmed.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::FORBIDDEN)
                .body("Cannot delete root collection".to_string())
                .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))?);
        }

        let folder_result = self
            .folder_service
            .find_by_path(storage_id, &format!("/{}", trimmed), user.id)
            .await;

        match folder_result {
            Ok(folder) => {
                self.folder_service
                    .delete_folder(filehub_core::types::id::FolderId::from(folder.id), user.id)
                    .await
                    .map_err(|e| AppError::internal(format!("Failed to delete folder: {}", e)))?;

                tracing::info!("Deleted folder via WebDAV: path='{}'", path);
            }
            Err(_) => {
                let file = self
                    .file_service
                    .find_by_path(storage_id, &format!("/{}", trimmed), user.id)
                    .await
                    .map_err(|_| AppError::not_found("Resource not found"))?;

                self.file_service
                    .delete_file(filehub_core::types::id::FileId::from(file.id), user.id)
                    .await
                    .map_err(|e| AppError::internal(format!("Failed to delete file: {}", e)))?;

                tracing::info!("Deleted file via WebDAV: path='{}'", path);
            }
        }

        Ok(Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(String::new())
            .map_err(|e| AppError::internal(format!("Failed to build response: {}", e)))?)
    }

    /// Convert an AppError to an HTTP response
    fn error_response(&self, error: &AppError) -> Response<String> {
        let status = match error.to_string().as_str() {
            s if s.contains("not found") => StatusCode::NOT_FOUND,
            s if s.contains("unauthorized") => StatusCode::UNAUTHORIZED,
            s if s.contains("forbidden") => StatusCode::FORBIDDEN,
            s if s.contains("conflict") || s.contains("already exists") => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        Response::builder()
            .status(status)
            .header("Content-Type", "text/plain")
            .body(error.to_string())
            .unwrap_or_else(|_| {
                let mut r = Response::new("Internal Server Error".to_string());
                *r.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                r
            })
    }
}
