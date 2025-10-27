use axum::body::Body;
use axum::http::{Request, StatusCode, header::AUTHORIZATION};
use chrono::Utc;
use tower::util::ServiceExt as _;
mod support;

#[tokio::test]
async fn e2e_list_and_revoke_sessions() {
    // build state and router so we can seed the session store directly
    let state = support::build_test_state().await;
    let store = state.services.session_revocation_store();

    // user id used by the DummyTokenManager::session_user() is 4
    let user_id = 4i64;
    let now = Utc::now().timestamp();

    // seed two sessions for the user
    store
        .set_session_metadata(user_id, "sid-1", Some("ua-1"), Some("10.0.0.1"), now)
        .await
        .expect("set meta");
    store
        .set_session_metadata(user_id, "sid-2", Some("ua-2"), Some("10.0.0.2"), now - 3600)
        .await
        .expect("set meta");

    let app = mokkan_core::presentation::http::routes::build_router_with_rate_limiter(
        state.clone(),
        false,
    );

    // list sessions as the sessioned user (DummyTokenManager::SESSION_TOKEN -> session_id = sid-1, user id = 4)
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/auth/sessions")
        .header(AUTHORIZATION, format!("Bearer {}", support::SESSION_TOKEN))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let (_headers, json) = to_json_async!(resp).await;
    assert!(json.is_array(), "expected array of sessions");
    let arr = json.as_array().unwrap();
    // Expect at least the two sessions we created
    let ids: Vec<String> = arr
        .iter()
        .filter_map(|v| {
            v.get("session_id")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
        })
        .collect();

    assert!(ids.contains(&"sid-1".to_string()));
    assert!(ids.contains(&"sid-2".to_string()));

    // Revoke sid-2 as owner
    let req = Request::builder()
        .method("DELETE")
        .uri("/api/v1/auth/sessions/sid-2")
        .header(AUTHORIZATION, format!("Bearer {}", support::SESSION_TOKEN))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // ensure the store reports the session revoked and it's removed from list
    let revoked = store.is_revoked("sid-2").await.expect("is_revoked");
    assert!(revoked, "sid-2 should be revoked");

    let remaining = store.list_sessions_for_user(user_id).await.expect("list");
    assert!(
        !remaining.contains(&"sid-2".to_string()),
        "sid-2 should have been removed from user's sessions"
    );
}
