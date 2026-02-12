//! File upload service — simple (single request) and chunked upload flows.

use std::sync::Arc;

use bytes::Bytes;
use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use filehub_auth::acl::EffectivePermissionResolver;
use filehub_core::config::StorageConfig;
use filehub_core::error::AppError;
use filehub_database::repositories::file::FileRepository;
use filehub_database::repositories::folder::FolderRepository;
use filehub_entity::file::{CreateFile, File};
use filehub_entity::permission::{AclPermission, ResourceType};
use filehub_plugin::hooks::definitions::{HookPayload, HookPoint};
use filehub_plugin::manager::PluginManager;
use filehub_storage::manager::StorageManager;

use crate::context::RequestContext;

/// Handles both simple and chunked file uploads.
#[derive(Clone)]
pub struct UploadService {
    /// File repository.
    file_repo: Arc<FileRepository>,
    /// Folder repository.
    folder_repo: Arc<FolderRepository>,
    /// Storage manager.
    storage: Arc<StorageManager>,
    /// Permission resolver.
    perm_resolver: Arc<EffectivePermissionResolver>,
    /// Storage configuration.
    config: StorageConfig,
    /// Plugin manager for firing hooks.
    plugin_manager: Arc<PluginManager>,
}

impl std::fmt::Debug for UploadService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UploadService").finish()
    }
}

/// Request for initiating a chunked upload.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InitiateUploadRequest {
    /// Target folder ID.
    pub folder_id: Uuid,
    /// File name.
    pub file_name: String,
    /// Total file size in bytes.
    pub file_size: i64,
    /// MIME type.
    pub mime_type: Option<String>,
    /// Expected SHA-256 checksum.
    pub checksum_sha256: Option<String>,
}

/// Response from initiating a chunked upload.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InitiateUploadResponse {
    /// Upload session ID.
    pub upload_id: Uuid,
    /// Size of each chunk.
    pub chunk_size: i64,
    /// Total number of chunks to upload.
    pub total_chunks: i32,
}

/// Simple upload parameters (single request with full file body).
#[derive(Debug, Clone)]
pub struct SimpleUploadParams {
    /// Target folder ID.
    pub folder_id: Uuid,
    /// File name.
    pub file_name: String,
    /// MIME type.
    pub mime_type: Option<String>,
    /// File content bytes.
    pub data: Bytes,
}

impl UploadService {
    /// Creates a new upload service.
    pub fn new(
        file_repo: Arc<FileRepository>,
        folder_repo: Arc<FolderRepository>,
        storage: Arc<StorageManager>,
        perm_resolver: Arc<EffectivePermissionResolver>,
        config: StorageConfig,
        plugin_manager: Arc<PluginManager>,
    ) -> Self {
        Self {
            file_repo,
            folder_repo,
            storage,
            perm_resolver,
            config,
            plugin_manager,
        }
    }

    /// Performs a simple (single-request) file upload.
    pub async fn simple_upload(
        &self,
        ctx: &RequestContext,
        params: SimpleUploadParams,
    ) -> Result<File, AppError> {
        // Check size limit
        if params.data.len() as u64 > self.config.max_upload_size_bytes {
            return Err(AppError::validation(format!(
                "File exceeds maximum upload size of {} bytes",
                self.config.max_upload_size_bytes
            )));
        }

        // Verify folder exists and user has editor permission
        let folder = self
            .folder_repo
            .find_by_id(params.folder_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("Target folder not found"))?;

        self.perm_resolver
            .require_permission(
                ctx.user_id,
                &ctx.role,
                ResourceType::Folder,
                params.folder_id,
                folder.owner_id,
                folder.parent_id,
                AclPermission::Editor,
            )
            .await?;

        // Check for name conflict
        if self
            .file_repo
            .find_by_folder_and_name(params.folder_id, &params.file_name)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .is_some()
        {
            return Err(AppError::conflict(format!(
                "A file named '{}' already exists in this folder",
                params.file_name
            )));
        }

        // Write to storage
        let file_id = Uuid::new_v4();
        let storage_path = format!("{}/{}/{}", folder.path, file_id, params.file_name);

        self.storage
            .write(&folder.storage_id, &storage_path, params.data.clone())
            .await
            .map_err(|e| AppError::internal(format!("Storage write failed: {e}")))?;

        // Create file record
        let file_record = CreateFile {
            folder_id: params.folder_id,
            storage_id: folder.storage_id,
            name: params.file_name,
            storage_path,
            mime_type: params.mime_type,
            size_bytes: params.data.len() as i64,
            checksum_sha256: None,
            metadata: Some(serde_json::json!({})),
            owner_id: ctx.user_id,
        };

        let file = self
            .file_repo
            .create(&file_record)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create file record: {e}")))?;

        info!(
            user_id = %ctx.user_id,
            file_id = %file.id,
            name = %file.name,
            size = file.size_bytes,
            "Simple upload completed"
        );

        // Fire AfterUpload hook
        let mut payload = HookPayload::new(HookPoint::AfterUpload)
            .with_uuid("file_id", file.id)
            .with_uuid("folder_id", file.folder_id)
            .with_uuid("owner_id", file.owner_id)
            .with_string("name", &file.name)
            .with_string("storage_path", &file.storage_path)
            .with_int("size_bytes", file.size_bytes);

        if let Some(mime) = &file.mime_type {
            payload = payload.with_string("mime_type", mime);
        }

        self.plugin_manager
            .dispatcher()
            .fire_and_forget(&payload)
            .await;

        Ok(file)
    }

    /// Initiates a chunked upload session.
    pub async fn initiate_chunked_upload(
        &self,
        ctx: &RequestContext,
        req: InitiateUploadRequest,
    ) -> Result<InitiateUploadResponse, AppError> {
        // Check size limit
        if req.file_size as u64 > self.config.max_upload_size_bytes {
            return Err(AppError::validation(format!(
                "File exceeds maximum upload size of {} bytes",
                self.config.max_upload_size_bytes
            )));
        }

        // Verify folder and permission
        let folder = self
            .folder_repo
            .find_by_id(req.folder_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("Target folder not found"))?;

        self.perm_resolver
            .require_permission(
                ctx.user_id,
                &ctx.role,
                ResourceType::Folder,
                req.folder_id,
                folder.owner_id,
                folder.parent_id,
                AclPermission::Editor,
            )
            .await?;

        let chunk_size = self.config.chunk_size_bytes as i64;
        let total_chunks = ((req.file_size as f64) / (chunk_size as f64)).ceil() as i32;
        let total_chunks = if total_chunks == 0 { 1 } else { total_chunks };

        let upload_id = Uuid::new_v4();
        let temp_path = format!("temp/uploads/{}", upload_id);
        let now = Utc::now();
        let expires_at = now + chrono::Duration::hours(24);

        self.file_repo
            .create_chunked_upload(
                ctx.user_id,
                folder.storage_id,
                req.folder_id,
                &req.file_name,
                req.file_size,
                req.mime_type.as_deref(),
                chunk_size as i32,
                total_chunks,
                req.checksum_sha256.as_deref(),
                &temp_path,
                expires_at,
            )
            .await
            .map_err(|e| AppError::internal(format!("Failed to create upload session: {e}")))?;

        info!(
            user_id = %ctx.user_id,
            upload_id = %upload_id,
            total_chunks = total_chunks,
            "Chunked upload initiated"
        );

        Ok(InitiateUploadResponse {
            upload_id,
            chunk_size,
            total_chunks,
        })
    }

    /// Uploads a single chunk.
    pub async fn upload_chunk(
        &self,
        ctx: &RequestContext,
        upload_id: Uuid,
        chunk_number: i32,
        data: Bytes,
    ) -> Result<(), AppError> {
        let upload = self
            .file_repo
            .find_chunked_upload(upload_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("Upload session not found"))?;

        if upload.user_id != ctx.user_id {
            return Err(AppError::forbidden(
                "Upload session belongs to another user",
            ));
        }

        if upload.status != "uploading" {
            return Err(AppError::conflict(
                "Upload session is not in uploading state",
            ));
        }

        if chunk_number < 0 || chunk_number >= upload.total_chunks {
            return Err(AppError::validation(format!(
                "Invalid chunk number: {chunk_number} (total: {})",
                upload.total_chunks
            )));
        }

        // Write chunk to temp storage
        let chunk_path = format!("{}/chunk_{:06}", upload.temp_path, chunk_number);
        self.storage
            .write(&upload.storage_id, &chunk_path, data)
            .await
            .map_err(|e| AppError::internal(format!("Failed to write chunk: {e}")))?;

        // Update uploaded_chunks list
        self.file_repo
            .add_uploaded_chunk(upload_id, chunk_number)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update chunk status: {e}")))?;

        info!(
            upload_id = %upload_id,
            chunk = chunk_number,
            "Chunk uploaded"
        );

        Ok(())
    }

    /// Completes a chunked upload — verifies all chunks and assembles the file.
    pub async fn complete_chunked_upload(
        &self,
        ctx: &RequestContext,
        upload_id: Uuid,
    ) -> Result<File, AppError> {
        let upload = self
            .file_repo
            .find_chunked_upload(upload_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("Upload session not found"))?;

        if upload.user_id != ctx.user_id {
            return Err(AppError::forbidden(
                "Upload session belongs to another user",
            ));
        }

        if upload.status != "uploading" {
            return Err(AppError::conflict("Upload is not in uploading state"));
        }

        // Verify all chunks are present
        let uploaded: Vec<i32> =
            serde_json::from_value(upload.uploaded_chunks.clone()).unwrap_or_default();

        let expected: Vec<i32> = (0..upload.total_chunks).collect();
        let mut sorted_uploaded = uploaded.clone();
        sorted_uploaded.sort();
        sorted_uploaded.dedup();

        if sorted_uploaded != expected {
            let missing: Vec<i32> = expected
                .iter()
                .filter(|c| !sorted_uploaded.contains(c))
                .copied()
                .collect();
            return Err(AppError::validation(format!(
                "Missing chunks: {:?}",
                missing
            )));
        }

        // Assemble chunks into final file
        let folder = self
            .folder_repo
            .find_by_id(upload.target_folder_id)
            .await
            .map_err(|e| AppError::internal(format!("Database error: {e}")))?
            .ok_or_else(|| AppError::not_found("Target folder not found"))?;

        let file_id = Uuid::new_v4();
        let storage_path = format!("{}/{}/{}", folder.path, file_id, upload.file_name);

        // Read all chunks and assemble
        let mut assembled = Vec::with_capacity(upload.file_size as usize);
        for chunk_num in 0..upload.total_chunks {
            let chunk_path = format!("{}/chunk_{:06}", upload.temp_path, chunk_num);
            let chunk_data = self
                .storage
                .read(&upload.storage_id, &chunk_path)
                .await
                .map_err(|e| {
                    AppError::internal(format!("Failed to read chunk {chunk_num}: {e}"))
                })?;
            assembled.extend_from_slice(&chunk_data);
        }

        // Write assembled file
        self.storage
            .write(
                &upload.storage_id,
                &storage_path,
                Bytes::from(assembled.clone()),
            )
            .await
            .map_err(|e| AppError::internal(format!("Failed to write assembled file: {e}")))?;

        // Create file record
        let file_record = CreateFile {
            folder_id: upload.target_folder_id,
            storage_id: upload.storage_id,
            name: upload.file_name.clone(),
            storage_path,
            mime_type: upload.mime_type.clone(),
            size_bytes: assembled.len() as i64,
            checksum_sha256: upload.checksum_sha256.clone(),
            metadata: Some(serde_json::json!({})),
            owner_id: ctx.user_id,
        };

        let file = self
            .file_repo
            .create(&file_record)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create file record: {e}")))?;

        // Mark upload as completed
        self.file_repo
            .complete_chunked_upload(upload_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to complete upload: {e}")))?;

        // Cleanup temp chunks (best effort)
        for chunk_num in 0..upload.total_chunks {
            let chunk_path = format!("{}/chunk_{:06}", upload.temp_path, chunk_num);
            let _ = self.storage.delete(&upload.storage_id, &chunk_path).await;
        }

        info!(
            user_id = %ctx.user_id,
            file_id = %file.id,
            name = %file.name,
            size = file.size_bytes,
            chunks = upload.total_chunks,
            "Chunked upload completed and assembled"
        );

        // Fire AfterUpload hook
        let mut payload = HookPayload::new(HookPoint::AfterUpload)
            .with_uuid("file_id", file.id)
            .with_uuid("folder_id", file.folder_id)
            .with_uuid("owner_id", file.owner_id)
            .with_string("name", &file.name)
            .with_string("storage_path", &file.storage_path)
            .with_int("size_bytes", file.size_bytes);

        if let Some(mime) = &file.mime_type {
            payload = payload.with_string("mime_type", mime);
        }

        self.plugin_manager
            .dispatcher()
            .fire_and_forget(&payload)
            .await;

        Ok(file)
    }
}
