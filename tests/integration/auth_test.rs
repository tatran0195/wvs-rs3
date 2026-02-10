//! Integration tests for authentication flow.

mod helpers;

use http::StatusCode;

#[tokio::test]
async fn test_login_success() {
    let app = helpers::TestApp::new().await;
    app.create_test_user("testuser", "password123", "viewer")
        .await;

    let response = app
        .request(
            "POST",
            "/api/auth/login",
            Some(serde_json::json!({
                "username": "testuser",
                "password": "password123",
            })),
            None,
        )
        .await;

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.body.get("access_token").is_some());
    assert!(response.body.get("refresh_token").is_some());
}

#[tokio::test]
async fn test_login_invalid_password() {
    let app = helpers::TestApp::new().await;
    app.create_test_user("testuser2", "password123", "viewer")
        .await;

    let response = app
        .request(
            "POST",
            "/api/auth/login",
            Some(serde_json::json!({
                "username": "testuser2",
                "password": "wrongpassword",
            })),
            None,
        )
        .await;

    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_nonexistent_user() {
    let app = helpers::TestApp::new().await;

    let response = app
        .request(
            "POST",
            "/api/auth/login",
            Some(serde_json::json!({
                "username": "nobody",
                "password": "password123",
            })),
            None,
        )
        .await;

    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_me_authenticated() {
    let app = helpers::TestApp::new().await;
    app.create_test_user("meuser", "password123", "admin").await;
    let token = app.login("meuser", "password123").await;

    let response = app.request("GET", "/api/auth/me", None, Some(&token)).await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        response.body.get("username").unwrap().as_str().unwrap(),
        "meuser"
    );
}

#[tokio::test]
async fn test_me_unauthenticated() {
    let app = helpers::TestApp::new().await;

    let response = app.request("GET", "/api/auth/me", None, None).await;

    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_logout() {
    let app = helpers::TestApp::new().await;
    app.create_test_user("logoutuser", "password123", "viewer")
        .await;
    let token = app.login("logoutuser", "password123").await;

    let response = app
        .request("POST", "/api/auth/logout", None, Some(&token))
        .await;

    assert_eq!(response.status, StatusCode::OK);

    // Token should now be invalid
    let response = app.request("GET", "/api/auth/me", None, Some(&token)).await;
    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_refresh_token() {
    let app = helpers::TestApp::new().await;
    app.create_test_user("refreshuser", "password123", "viewer")
        .await;

    let login_resp = app
        .request(
            "POST",
            "/api/auth/login",
            Some(serde_json::json!({
                "username": "refreshuser",
                "password": "password123",
            })),
            None,
        )
        .await;

    let refresh_token = login_resp
        .body
        .get("refresh_token")
        .unwrap()
        .as_str()
        .unwrap();

    let response = app
        .request(
            "POST",
            "/api/auth/refresh",
            Some(serde_json::json!({
                "refresh_token": refresh_token,
            })),
            None,
        )
        .await;

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.body.get("access_token").is_some());
}
