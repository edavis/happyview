mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

use common::app::TestApp;

async fn user_id_for_did(app: &TestApp, did: &str) -> String {
    let sql = happyview::db::adapt_sql(
        "SELECT id FROM happyview_users WHERE did = ?",
        app.state.db_backend,
    );
    let row: (String,) = sqlx::query_as(&sql)
        .bind(did)
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    row.0
}

async fn is_super(app: &TestApp, user_id: &str) -> bool {
    let sql = happyview::db::adapt_sql(
        "SELECT is_super FROM happyview_users WHERE id = ?",
        app.state.db_backend,
    );
    let row: (i32,) = sqlx::query_as(&sql)
        .bind(user_id)
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    row.0 != 0
}

async fn insert_user(app: &TestApp, did: &str) -> String {
    let id = Uuid::new_v4().to_string();
    let sql = happyview::db::adapt_sql(
        "INSERT INTO happyview_users (id, did, is_super, created_at) VALUES (?, ?, ?, ?)",
        app.state.db_backend,
    );
    sqlx::query(&sql)
        .bind(&id)
        .bind(did)
        .bind(0_i32)
        .bind(happyview::db::now_rfc3339())
        .execute(&app.state.db)
        .await
        .unwrap();
    id
}

fn transfer_req(app: &TestApp, target_user_id: &str) -> Request<Body> {
    let (name, value) = app.admin_cookie();
    Request::builder()
        .method("POST")
        .uri("/admin/users/transfer-super")
        .header(name, value)
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "target_user_id": target_user_id }).to_string(),
        ))
        .unwrap()
}

/// A successful transfer promotes the target (with all permissions) and demotes
/// the previous super.
#[tokio::test]
#[serial]
async fn transfer_super_moves_super_and_grants_permissions() {
    common::require_db!();
    let app = TestApp::new().await;
    let admin_id = user_id_for_did(&app, &app.admin_did).await;
    let target_id = insert_user(&app, "did:plc:new-super").await;

    let resp = app
        .router
        .clone()
        .oneshot(transfer_req(&app, &target_id))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    assert!(is_super(&app, &target_id).await, "target should be super");
    assert!(
        !is_super(&app, &admin_id).await,
        "previous super should be demoted"
    );

    let sql = happyview::db::adapt_sql(
        "SELECT COUNT(*) FROM happyview_user_permissions WHERE user_id = ?",
        app.state.db_backend,
    );
    let count: (i64,) = sqlx::query_as(&sql)
        .bind(&target_id)
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert!(count.0 > 0, "target should have permissions granted");
}

/// Transferring to a non-existent user fails without demoting the current super
/// — the transaction rolls back, so the instance is never left without a super.
#[tokio::test]
#[serial]
async fn transfer_super_to_missing_user_preserves_current_super() {
    common::require_db!();
    let app = TestApp::new().await;
    let admin_id = user_id_for_did(&app, &app.admin_did).await;

    let resp = app
        .router
        .clone()
        .oneshot(transfer_req(&app, "does-not-exist"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert!(
        is_super(&app, &admin_id).await,
        "current super must be preserved when the transfer fails (no lockout)"
    );
}
