use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use incident_commander::{
    api::AppState,
    build_router,
    infra,
};
use serde_json::Value;
use tower::util::ServiceExt;

fn test_db_url() -> String {
    "sqlite::memory:".to_string()
}

async fn setup_app(api_key: Option<String>) -> axum::Router {
    let db_url = test_db_url();
    let db = infra::connect_db(&db_url).await.expect("connect test db");
    infra::init_db(&db).await.expect("init test db");
    build_router(AppState { db, api_key })
}

#[tokio::test]
async fn health_returns_ok() {
    let app = setup_app(None).await;

    let res = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn auth_required_when_api_key_set() {
    let app = setup_app(Some("secret123".into())).await;

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/incidents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_incident_and_fetch_by_id() {
    let app = setup_app(Some("secret123".into())).await;

    let create_payload = r#"{
        "title":"DB latency",
        "description":"p99 above SLO",
        "severity":"high"
    }"#;

    let create_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/incidents")
                .header("content-type", "application/json")
                .header("x-api-key", "secret123")
                .body(Body::from(create_payload))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_res.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(create_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: Value = serde_json::from_slice(&body).unwrap();
    let id = created["id"].as_str().unwrap();

    let get_res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/incidents/{}", id))
                .header("x-api-key", "secret123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_res.status(), StatusCode::OK);
}

#[tokio::test]
async fn create_incident_empty_title_returns_bad_request() {
    let app = setup_app(None).await;

    let payload = r#"{
        "title":"   ",
        "description":"invalid",
        "severity":"low"
    }"#;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/incidents")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_non_existing_incident_returns_not_found() {
    let app = setup_app(None).await;
    let id = "00000000-0000-0000-0000-000000000000";

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/incidents/{}", id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn created_incident_contains_created_timeline_event() {
    let app = setup_app(None).await;

    let payload = r#"{
        "title":"API 5xx spike",
        "description":"gateway errors",
        "severity":"critical"
    }"#;

    let create_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/incidents")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(create_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: Value = serde_json::from_slice(&body).unwrap();
    let id = created["id"].as_str().unwrap();

    let timeline_res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/incidents/{}/timeline", id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(timeline_res.status(), StatusCode::OK);

    let timeline_body = axum::body::to_bytes(timeline_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let timeline: Value = serde_json::from_slice(&timeline_body).unwrap();

    assert!(timeline.as_array().unwrap().iter().any(|e| e["event_type"] == "created"));
}
