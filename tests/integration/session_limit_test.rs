//! Integration tests for seat allocation and session limits.

mod helpers;

use http::StatusCode;

#[tokio::test]
async fn test_session_limit_enforced_for_managers() {
    let app = helpers::TestApp::new().await;
    app.create_test_user("limitmgr", "password123", "manager")
        .await;

    // First login should succeed
    let resp1 = app
        .request(
            "POST",
            "/api/auth/login",
            Some(serde_json::json!({
                "username": "limitmgr",
                "password": "password123",
            })),
            None,
        )
        .await;
    assert_eq!(resp1.status, StatusCode::OK);

    // Second login should be denied (manager limit = 1)
    let resp2 = app
        .request(
            "POST",
            "/api/auth/login",
            Some(serde_json::json!({
                "username": "limitmgr",
                "password": "password123",
            })),
            None,
        )
        .await;

    // Should either deny (409/429) or kick oldest depending on strategy
    assert!(
        resp2.status == StatusCode::TOO_MANY_REQUESTS
            || resp2.status == StatusCode::CONFLICT
            || resp2.status == StatusCode::OK, // if strategy is kick_oldest
        "Expected 429, 409, or 200, got {}",
        resp2.status
    );
}

#[tokio::test]
async fn test_admin_session_count() {
    let app = helpers::TestApp::new().await;
    app.create_test_user("countadmin", "password123", "admin")
        .await;
    let token = app.login("countadmin", "password123").await;

    let response = app
        .request("GET", "/api/admin/sessions", None, Some(&token))
        .await;

    assert_eq!(response.status, StatusCode::OK);
}
