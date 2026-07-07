mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;

async fn response_json(resp: axum::http::Response<Body>) -> serde_json::Value {
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap_or(json!(null))
}

/// With an insecure SESSION_SECRET, starting a cookie login must fail loudly
/// (503 ServerMisconfigured) rather than mint a forgeable cookie session.
#[tokio::test]
#[serial]
async fn insecure_secret_login_returns_503() {
    common::require_db!();
    let mut app = common::app::TestApp::new().await;
    app.set_insecure_session_secret();

    let req = Request::builder()
        .method("GET")
        .uri("/auth/login?handle=alice.test")
        .header("host", "127.0.0.1")
        .body(Body::empty())
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = response_json(resp).await;
    assert_eq!(body["error"], "ServerMisconfigured");
}

/// A cookie-authenticated admin request must be rejected with a clear 503 when
/// the session secret is insecure — the cookie signature can't be trusted.
#[tokio::test]
#[serial]
async fn insecure_secret_admin_cookie_returns_503() {
    common::require_db!();
    let mut app = common::app::TestApp::new().await;
    app.set_insecure_session_secret();

    let (name, value) = app.admin_cookie();
    let req = Request::builder()
        .method("GET")
        .uri("/admin/lexicons")
        .header("host", "127.0.0.1")
        .header(name, value)
        .body(Body::empty())
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = response_json(resp).await;
    assert_eq!(body["error"], "ServerMisconfigured");
}

/// Anonymous / public XRPC traffic must keep working when the session secret is
/// insecure — even for a client that happens to carry a stale session cookie.
/// The cookie is ignored (treated as anonymous), so the response is never the
/// misconfiguration 503.
#[tokio::test]
#[serial]
async fn insecure_secret_xrpc_ignores_cookie() {
    common::require_db!();
    let mut app = common::app::TestApp::new().await;

    // Capture a validly-signed cookie *before* flipping to the insecure state.
    let (name, value) = app.admin_cookie();
    app.set_insecure_session_secret();

    let req = Request::builder()
        .method("POST")
        .uri("/xrpc/com.example.test.procedure")
        .header("host", "127.0.0.1")
        .header("content-type", "application/json")
        .header(name, value)
        .body(Body::from("{}"))
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_ne!(
        resp.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "the misconfiguration gate must not block XRPC traffic; the cookie should be ignored"
    );
}

/// `/config` surfaces the misconfiguration so the dashboard can explain it.
#[tokio::test]
#[serial]
async fn config_endpoint_reports_errors_when_insecure() {
    common::require_db!();
    let mut app = common::app::TestApp::new().await;
    app.set_insecure_session_secret();

    let req = Request::builder()
        .method("GET")
        .uri("/config")
        .header("host", "127.0.0.1")
        .body(Body::empty())
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    let errors = body["configErrors"].as_array().expect("configErrors array");
    assert!(
        !errors.is_empty(),
        "configErrors should list the insecure SESSION_SECRET"
    );
}

/// A correctly configured instance reports no config errors and serves login.
#[tokio::test]
#[serial]
async fn config_endpoint_reports_no_errors_when_healthy() {
    common::require_db!();
    let app = common::app::TestApp::new().await;

    let req = Request::builder()
        .method("GET")
        .uri("/config")
        .header("host", "127.0.0.1")
        .body(Body::empty())
        .unwrap();

    let resp = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    let errors = body["configErrors"].as_array().expect("configErrors array");
    assert!(
        errors.is_empty(),
        "a healthy instance should report no config errors"
    );
}
