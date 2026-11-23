use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use brickxinator_app::{AppState, router};
use brickxinator_core::ledger::Id;
use brickxinator_infra::sqlite::Store;
use std::time::{Duration, Instant};
use tower::ServiceExt;

fn fixture() -> (tempfile::TempDir, AppState) {
    let root = tempfile::tempdir().unwrap();
    let database = root.path().join("database.sqlite");
    drop(Store::create(&database).unwrap());
    let state = AppState::new(
        database,
        root.path().to_owned(),
        "http://127.0.0.1:4217".into(),
        90,
    );
    (root, state)
}

#[tokio::test]
async fn host_origin_and_token_are_enforced_before_commands() {
    let (_root, state) = fixture();
    for (host, origin, token, expected) in [
        ("evil.example", None, None, StatusCode::FORBIDDEN),
        (
            "127.0.0.1:4217",
            Some("https://evil.example"),
            Some(state.token.as_str()),
            StatusCode::FORBIDDEN,
        ),
        (
            "127.0.0.1:4217",
            Some(state.origin.as_str()),
            None,
            StatusCode::FORBIDDEN,
        ),
        (
            "127.0.0.1:4217",
            None,
            Some(state.token.as_str()),
            StatusCode::FORBIDDEN,
        ),
        (
            "127.0.0.1:4217",
            Some(state.origin.as_str()),
            Some(state.token.as_str()),
            StatusCode::OK,
        ),
    ] {
        let mut request = Request::post("/api/v1/app-tabs/heartbeat")
            .header("host", host)
            .header("content-type", "application/json");
        if let Some(origin) = origin {
            request = request.header("origin", origin);
        }
        if let Some(token) = token {
            request = request.header("x-brickxinator-token", token);
        }
        let response = router(state.clone())
            .oneshot(
                request
                    .body(Body::from(format!(r#"{{"tab_id":"{}"}}"#, Id::new())))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    assert_eq!(state.tabs.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn initial_state_is_readable_with_security_headers() {
    let (_root, state) = fixture();
    let response = router(state)
        .oneshot(
            Request::get("/api/v1/state")
                .header("host", "127.0.0.1:4217")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    assert!(
        response.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("frame-ancestors 'none'")
    );
    let value: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(value["purchases"], serde_json::json!([]));
}

#[test]
fn live_tabs_prevent_exit_and_expired_tabs_allow_idle_shutdown() {
    let (_root, state) = fixture();
    *state.idle_since.lock().unwrap() = Instant::now() - Duration::from_secs(91);
    state.tabs.lock().unwrap().insert(Id::new(), Instant::now());
    assert!(!state.should_stop());
    state.tabs.lock().unwrap().clear();
    assert!(!state.should_stop());
    state
        .tabs
        .lock()
        .unwrap()
        .insert(Id::new(), Instant::now() - Duration::from_secs(31));
    *state.idle_since.lock().unwrap() = Instant::now() - Duration::from_secs(91);
    assert!(state.should_stop());
    assert!(state.tabs.lock().unwrap().is_empty());
}

#[tokio::test]
async fn quantities_above_javascript_integer_precision_are_decimal_strings() {
    use brickxinator_core::{
        inventory::Measure,
        ledger::{Body as Event, Operation},
        views::Search,
    };
    let (_root, state) = fixture();
    Store::open(&state.database)
        .unwrap()
        .commit(Operation {
            id: Id::new(),
            request_key: "large".into(),
            actor: "test".into(),
            reason: "exact measurement".into(),
            body: Event::Create {
                holding_id: Id::new(),
                pieces: Measure::Known(u64::MAX),
                weight_mg: Measure::Unknown,
            },
        })
        .unwrap();
    let request = Request::post("/api/v1/inventory")
        .header("host", "127.0.0.1:4217")
        .header("origin", &state.origin)
        .header("x-brickxinator-token", &state.token)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&Search::default()).unwrap()))
        .unwrap();
    let response = router(state).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(
        value["rows"][0]["holding"]["pieces"]["owned"]["Known"],
        u64::MAX.to_string()
    );
}
