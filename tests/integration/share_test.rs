//! Integration tests for share create and access.

mod helpers;

use http::StatusCode;

#[tokio::test]
async fn test_create_share() {
    let app = helpers::TestApp::new().await;
    app.create_test_user("shareuser", "password123", "admin")
        .await;
    let token = app.login("shareuser", "password123").await;

    // First create a storage and file to share
    let storage_id = create_test_storage(&app).await;
    let folder_id = create_test_folder(&app, &storage_id).await;
    let file_id = create_test_file(&app, &folder_id, &storage_id).await;

    let response = app
        .request(
            "POST",
            "/api/shares",
            Some(serde_json::json!({
                "resource_type": "file",
                "resource_id": file_id,
                "share_type": "public_link",
                "permission": "viewer",
            })),
            Some(&token),
        )
        .await;

    assert!(
        response.status == StatusCode::CREATED || response.status == StatusCode::OK,
        "Expected 201 or 200, got {}",
        response.status
    );

    if let Some(token_val) = response.body.get("token") {
        assert!(token_val.as_str().is_some());
    }
}

#[tokio::test]
async fn test_list_shares() {
    let app = helpers::TestApp::new().await;
    app.create_test_user("listshareuser", "password123", "admin")
        .await;
    let token = app.login("listshareuser", "password123").await;

    let response = app.request("GET", "/api/shares", None, Some(&token)).await;

    assert_eq!(response.status, StatusCode::OK);
}

async fn create_test_storage(app: &helpers::TestApp) -> String {
    let id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO storages (id, name, provider_type, config, status, is_default, created_at, updated_at)
           VALUES ($1, 'Test', 'local', '{"root_path":"/tmp/filehub_test/local"}', 'active', TRUE, NOW(), NOW())"#,
    )
    .bind(id)
    .execute(&app.db_pool)
    .await
    .expect("Failed to create storage");
    id.to_string()
}

async fn create_test_folder(app: &helpers::TestApp, storage_id: &str) -> String {
    let id = uuid::Uuid::new_v4();
    let sid: uuid::Uuid = storage_id.parse().unwrap();
    sqlx::query(
        r#"INSERT INTO folders (id, storage_id, name, path, depth, owner_id, created_at, updated_at)
           VALUES ($1, $2, 'test', '/test', 0, '00000000-0000-0000-0000-000000000001', NOW(), NOW())"#,
    )
    .bind(id)
    .bind(sid)
    .execute(&app.db_pool)
    .await
    .expect("Failed to create folder");
    id.to_string()
}

async fn create_test_file(app: &helpers::TestApp, folder_id: &str, storage_id: &str) -> String {
    let id = uuid::Uuid::new_v4();
    let fid: uuid::Uuid = folder_id.parse().unwrap();
    let sid: uuid::Uuid = storage_id.parse().unwrap();
    sqlx::query(
        r#"INSERT INTO files (id, folder_id, storage_id, name, storage_path, mime_type, size_bytes, owner_id, created_at, updated_at)
           VALUES ($1, $2, $3, 'share_test.txt', '/test/share_test.txt', 'text/plain', 100, '00000000-0000-0000-0000-000000000001', NOW(), NOW())"#,
    )
    .bind(id)
    .bind(fid)
    .bind(sid)
    .execute(&app.db_pool)
    .await
    .expect("Failed to create file");
    id.to_string()
}
