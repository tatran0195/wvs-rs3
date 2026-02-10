//! Integration tests for file operations.

mod helpers;

use http::StatusCode;

#[tokio::test]
async fn test_list_files_authenticated() {
    let app = helpers::TestApp::new().await;
    app.create_test_user("fileuser", "password123", "admin")
        .await;
    let token = app.login("fileuser", "password123").await;

    let response = app.request("GET", "/api/files", None, Some(&token)).await;

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.body.get("items").is_some());
}

#[tokio::test]
async fn test_list_files_unauthenticated() {
    let app = helpers::TestApp::new().await;

    let response = app.request("GET", "/api/files", None, None).await;

    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_get_file_not_found() {
    let app = helpers::TestApp::new().await;
    app.create_test_user("fileuser2", "password123", "admin")
        .await;
    let token = app.login("fileuser2", "password123").await;

    let response = app
        .request(
            "GET",
            "/api/files/00000000-0000-0000-0000-999999999999",
            None,
            Some(&token),
        )
        .await;

    assert_eq!(response.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_search_files() {
    let app = helpers::TestApp::new().await;
    app.create_test_user("searchuser", "password123", "admin")
        .await;
    let token = app.login("searchuser", "password123").await;

    let response = app
        .request("GET", "/api/files/search?q=test", None, Some(&token))
        .await;

    assert_eq!(response.status, StatusCode::OK);
}
