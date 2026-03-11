use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use incident_commander::{api::AppState, build_router, infra};
use serde_json::Value;
use tower::util::ServiceExt;

async fn setup_app() -> axum::Router {
    let db = infra::connect_db("sqlite::memory:").await.expect("connect test db");
    infra::init_db(&db).await.expect("init test db");
    build_router(AppState { db, api_key: None })
}

async fn create_incident(app: &axum::Router, title: &str, severity: &str) -> String {
    let payload = format!(
        r#"{{"title":"{}","description":"desc","severity":"{}"}}"#,
        title, severity
    );

    let res = app
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

    assert_eq!(res.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    json["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn status_transition_flow_and_invalid_transition() {
    let app = setup_app().await;
    let id = create_incident(&app, "DB outage", "critical").await;

    let ack_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/incidents/{}/ack", id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ack_res.status(), StatusCode::OK);

    let resolve_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/incidents/{}/resolve", id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resolve_res.status(), StatusCode::OK);

    let invalid_back_to_ack = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/incidents/{}/ack", id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(invalid_back_to_ack.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn notes_append_to_timeline() {
    let app = setup_app().await;
    let id = create_incident(&app, "API errors", "high").await;

    let note_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/incidents/{}/notes", id))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"note":"Investigating ingress controller"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(note_res.status(), StatusCode::OK);

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

    let body = axum::body::to_bytes(timeline_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let timeline: Value = serde_json::from_slice(&body).unwrap();

    assert!(timeline
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["event_type"] == "note_added"));
}

#[tokio::test]
async fn list_filters_and_pagination_work() {
    let app = setup_app().await;

    let _a = create_incident(&app, "A", "high").await;
    let b = create_incident(&app, "B", "low").await;
    let _c = create_incident(&app, "C", "high").await;

    let ack_b = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/incidents/{}/ack", b))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ack_b.status(), StatusCode::OK);

    let filtered_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/incidents?status=open&severity=high&limit=10&offset=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(filtered_res.status(), StatusCode::OK);

    let filtered_body = axum::body::to_bytes(filtered_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let filtered: Value = serde_json::from_slice(&filtered_body).unwrap();
    let arr = filtered.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert!(arr.iter().all(|i| i["status"] == "open" && i["severity"] == "high"));

    let paged_res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/incidents?limit=1&offset=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(paged_res.status(), StatusCode::OK);

    let paged_body = axum::body::to_bytes(paged_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let paged: Value = serde_json::from_slice(&paged_body).unwrap();
    assert_eq!(paged.as_array().unwrap().len(), 1);
}
