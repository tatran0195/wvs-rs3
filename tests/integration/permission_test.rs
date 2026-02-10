//! Integration tests for RBAC + ACL permissions.

mod helpers;

use http::StatusCode;

#[tokio::test]
async fn test_admin_can_access_admin_endpoints() {
    let app = helpers::TestApp::new().await;
    app.create_test_user("permadmin", "password123", "admin")
        .await;
    let token = app.login("permadmin", "password123").await;

    let response = app
        .request("GET", "/api/admin/users", None, Some(&token))
        .await;

    assert_eq!(response.status, StatusCode::OK);
}

#[tokio::test]
async fn test_viewer_cannot_access_admin_endpoints() {
    let app = helpers::TestApp::new().await;
    app.create_test_user("permviewer", "password123", "viewer")
        .await;
    let token = app.login("permviewer", "password123").await;

    let response = app
        .request("GET", "/api/admin/users", None, Some(&token))
        .await;

    assert_eq!(response.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_can_create_user() {
    let app = helpers::TestApp::new().await;
    app.create_test_user("creatoradmin", "password123", "admin")
        .await;
    let token = app.login("creatoradmin", "password123").await;

    let response = app
        .request(
            "POST",
            "/api/admin/users",
            Some(serde_json::json!({
                "username": "newuser",
                "password": "newpass123",
                "role": "viewer",
            })),
            Some(&token),
        )
        .await;

    assert!(
        response.status == StatusCode::CREATED || response.status == StatusCode::OK,
        "Expected 201 or 200, got {}",
        response.status
    );
}

#[tokio::test]
async fn test_viewer_cannot_create_user() {
    let app = helpers::TestApp::new().await;
    app.create_test_user("viewercreator", "password123", "viewer")
        .await;
    let token = app.login("viewercreator", "password123").await;

    let response = app
        .request(
            "POST",
            "/api/admin/users",
            Some(serde_json::json!({
                "username": "newuser2",
                "password": "newpass123",
                "role": "viewer",
            })),
            Some(&token),
        )
        .await;

    assert_eq!(response.status, StatusCode::FORBIDDEN);
}
