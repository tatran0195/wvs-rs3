//! Integration tests for WebSocket connection and messaging.

mod helpers;

use http::StatusCode;

#[tokio::test]
async fn test_ws_upgrade_without_token() {
    let app = helpers::TestApp::new().await;

    // WebSocket upgrade without token should fail
    let response = app.request("GET", "/ws", None, None).await;

    assert!(
        response.status == StatusCode::UNAUTHORIZED
            || response.status == StatusCode::BAD_REQUEST
            || response.status == StatusCode::UPGRADE_REQUIRED,
        "Expected 401, 400, or 426, got {}",
        response.status
    );
}

#[tokio::test]
async fn test_health_check() {
    let app = helpers::TestApp::new().await;

    let response = app.request("GET", "/api/health", None, None).await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body.get("status").unwrap().as_str().unwrap(), "ok");
}

#[tokio::test]
async fn test_detailed_health_check() {
    let app = helpers::TestApp::new().await;

    let response = app.request("GET", "/api/health/detailed", None, None).await;

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.body.get("database").is_some());
    assert!(response.body.get("cache").is_some());
}
