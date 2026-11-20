use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use axum::{
    Form, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
};
use serde::{Deserialize, Serialize};

use crate::{AppError, AppState, Result, auth::AuthUser, files::stream_file};

#[derive(Serialize, sqlx::FromRow)]
pub struct Share {
    id: i64,
    token: String,
    expires_at: Option<String>,
    has_password: bool,
    download_count: i64,
    created_at: String,
}

#[derive(Deserialize)]
pub struct NewShare {
    expires_in_days: Option<i64>,
    password: Option<String>,
}

#[derive(Serialize)]
pub struct PublicShare {
    name: String,
    size_bytes: i64,
    mime_type: String,
    expires_at: Option<String>,
    password_required: bool,
}

/// `?preview` serves the file without counting it as a download (image preview on the share page).
#[derive(Deserialize)]
pub struct Opts {
    preview: Option<String>,
}

#[derive(Deserialize)]
pub struct Unlock {
    password: String,
}

#[derive(sqlx::FromRow)]
struct Shared {
    id: i64,
    name: String,
    mime_type: String,
    size_bytes: i64,
    storage_path: String,
    password_hash: Option<String>,
    expires_at: Option<String>,
}

fn forbidden(msg: &str) -> AppError {
    AppError(StatusCode::FORBIDDEN, msg.into())
}

/// Live (unexpired) share by token; missing and expired are the same 404.
async fn find(app: &AppState, token: &str) -> Result<Shared> {
    sqlx::query_as(
        "SELECT s.id, f.name, f.mime_type, f.size_bytes, f.storage_path, s.password_hash, s.expires_at
         FROM shares s JOIN files f ON f.id = s.file_id
         WHERE s.token = ? AND (s.expires_at IS NULL OR s.expires_at > datetime('now'))",
    )
    .bind(token)
    .fetch_optional(&app.db)
    .await?
    .ok_or_else(AppError::not_found)
}

async fn stream(app: &AppState, s: Shared, count: bool) -> Result<Response> {
    // No Range here: a ranged share download would be counted once per chunk.
    let res = stream_file(app, &s.name, s.mime_type, s.size_bytes, &s.storage_path, None).await?;
    if count {
        sqlx::query("UPDATE shares SET download_count = download_count + 1 WHERE id = ?")
            .bind(s.id)
            .execute(&app.db)
            .await?;
    }
    Ok(res)
}

pub async fn create(
    State(app): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(file_id): Path<i64>,
    Json(n): Json<NewShare>,
) -> Result<(StatusCode, Json<Share>)> {
    if n.expires_in_days.is_some_and(|d| !(1..=365).contains(&d)) {
        return Err(AppError::bad("expiry must be 1-365 days"));
    }
    let password = n.password.filter(|p| !p.is_empty());
    if password.as_ref().is_some_and(|p| p.chars().count() < 4) {
        return Err(AppError::bad("password must be at least 4 characters"));
    }
    let hash = match password {
        None => None,
        Some(p) => Some(
            tokio::task::spawn_blocking(move || Argon2::default().hash_password(p.as_bytes()).map(|h| h.to_string()))
                .await
                .map_err(AppError::internal)?
                .map_err(AppError::internal)?,
        ),
    };
    // datetime('now', NULL) is NULL, so no expiry needs no special case.
    let share = sqlx::query_as(
        "INSERT INTO shares (token, file_id, user_id, password_hash, expires_at)
         SELECT ?, id, user_id, ?, datetime('now', '+' || ? || ' days') FROM files WHERE id = ? AND user_id = ?
         RETURNING id, token, expires_at, password_hash IS NOT NULL AS has_password, download_count, created_at",
    )
    .bind(uuid::Uuid::new_v4().simple().to_string())
    .bind(hash)
    .bind(n.expires_in_days)
    .bind(file_id)
    .bind(uid)
    .fetch_optional(&app.db)
    .await?
    .ok_or_else(AppError::not_found)?;
    Ok((StatusCode::CREATED, Json(share)))
}

pub async fn list(
    State(app): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(file_id): Path<i64>,
) -> Result<Json<Vec<Share>>> {
    let shares = sqlx::query_as(
        "SELECT id, token, expires_at, password_hash IS NOT NULL AS has_password, download_count, created_at
         FROM shares WHERE file_id = ? AND user_id = ? ORDER BY id DESC",
    )
    .bind(file_id)
    .bind(uid)
    .fetch_all(&app.db)
    .await?;
    Ok(Json(shares))
}

pub async fn revoke(State(app): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<i64>) -> Result<StatusCode> {
    let res = sqlx::query("DELETE FROM shares WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(uid)
        .execute(&app.db)
        .await?;
    match res.rows_affected() {
        0 => Err(AppError::not_found()),
        _ => Ok(StatusCode::NO_CONTENT),
    }
}

pub async fn info(State(app): State<AppState>, Path(token): Path<String>) -> Result<Json<PublicShare>> {
    let s = find(&app, &token).await?;
    Ok(Json(PublicShare {
        name: s.name,
        size_bytes: s.size_bytes,
        mime_type: s.mime_type,
        expires_at: s.expires_at,
        password_required: s.password_hash.is_some(),
    }))
}

pub async fn download(
    State(app): State<AppState>,
    Path(token): Path<String>,
    Query(o): Query<Opts>,
) -> Result<Response> {
    let s = find(&app, &token).await?;
    if s.password_hash.is_some() {
        return Err(forbidden("password required"));
    }
    stream(&app, s, o.preview.is_none()).await
}

pub async fn unlock(
    State(app): State<AppState>,
    Path(token): Path<String>,
    Form(u): Form<Unlock>,
) -> Result<Response> {
    let s = find(&app, &token).await?;
    if let Some(hash) = s.password_hash.clone() {
        let ok = tokio::task::spawn_blocking(move || {
            PasswordHash::new(&hash).is_ok_and(|h| Argon2::default().verify_password(u.password.as_bytes(), &h).is_ok())
        })
        .await
        .map_err(AppError::internal)?;
        if !ok {
            return Err(forbidden("wrong password"));
        }
    }
    stream(&app, s, true).await
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::StatusCode};
    use serde_json::{Value, json};

    use crate::tests::{Client, test_app, user};

    #[tokio::test]
    async fn shares_flow() {
        let (app, dir, _) = test_app(1024, None).await;
        let mut alice = user(&app, "alice").await;
        let mut bob = user(&app, "bob").await;
        let mut anon = Client { app: app.clone(), cookie: None };

        let (_, folder) = alice.json("POST", "/api/folders", json!({ "name": "Docs" })).await;
        let (_, file) = alice.upload(folder["id"].as_i64().unwrap(), "hello.txt", b"hello").await;
        let fid = file["id"].as_i64().unwrap();
        let shares_uri = format!("/api/files/{fid}/shares");

        // create + validation
        let (s, plain) = alice.json("POST", &shares_uri, json!({ "expires_in_days": null, "password": "" })).await;
        assert_eq!(s, StatusCode::CREATED);
        assert_eq!(plain["has_password"], false);
        assert_eq!(plain["expires_at"], Value::Null);
        let (s, timed) = alice.json("POST", &shares_uri, json!({ "expires_in_days": 7 })).await;
        assert_eq!(s, StatusCode::CREATED);
        assert!(timed["expires_at"].is_string());
        for bad in [json!({ "expires_in_days": 0 }), json!({ "expires_in_days": 366 }), json!({ "password": "abc" })] {
            assert_eq!(alice.json("POST", &shares_uri, bad).await.0, StatusCode::BAD_REQUEST);
        }
        let (_, list) = alice.json("GET", &shares_uri, json!({})).await;
        assert_eq!(list.as_array().unwrap().len(), 2);
        assert_eq!(list[0]["id"], timed["id"]);

        // other users can't touch alice's shares
        assert_eq!(bob.json("GET", &shares_uri, json!({})).await.1, json!([]));
        assert_eq!(bob.json("POST", &shares_uri, json!({})).await.0, StatusCode::NOT_FOUND);
        let revoke_timed = format!("/api/shares/{}", timed["id"]);
        assert_eq!(bob.json("DELETE", &revoke_timed, json!({})).await.0, StatusCode::NOT_FOUND);
        assert_eq!(anon.json("GET", &shares_uri, json!({})).await.0, StatusCode::UNAUTHORIZED);

        // anonymous metadata + download
        let token = plain["token"].as_str().unwrap();
        let (s, meta) = anon.json("GET", &format!("/api/s/{token}"), json!({})).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(meta, json!({ "name": "hello.txt", "size_bytes": 5, "mime_type": "text/plain", "expires_at": null, "password_required": false }));
        let (s, bytes) = anon.send("GET", &format!("/api/s/{token}/download"), "", Body::empty()).await;
        assert_eq!((s, bytes.as_slice()), (StatusCode::OK, &b"hello"[..]));
        assert_eq!(anon.json("GET", "/api/s/nope", json!({})).await.0, StatusCode::NOT_FOUND);

        // revoke
        assert_eq!(alice.json("DELETE", &revoke_timed, json!({})).await.0, StatusCode::NO_CONTENT);
        assert_eq!(alice.json("DELETE", &revoke_timed, json!({})).await.0, StatusCode::NOT_FOUND);
        let timed_token = timed["token"].as_str().unwrap();
        assert_eq!(anon.json("GET", &format!("/api/s/{timed_token}"), json!({})).await.0, StatusCode::NOT_FOUND);

        // expired = 404, still listed for the owner
        let (_, exp) = alice.json("POST", &shares_uri, json!({ "expires_in_days": 1 })).await;
        let db = sqlx::SqlitePool::connect(&format!("sqlite://{}", dir.join("app.db").display())).await.unwrap();
        sqlx::query("UPDATE shares SET expires_at = datetime('now', '-1 day') WHERE id = ?")
            .bind(exp["id"].as_i64())
            .execute(&db)
            .await
            .unwrap();
        let exp_token = exp["token"].as_str().unwrap();
        let (s, body) = anon.json("GET", &format!("/api/s/{exp_token}"), json!({})).await;
        assert_eq!((s, body), (StatusCode::NOT_FOUND, json!({ "error": "not found" })));
        let (s, _) = anon.send("GET", &format!("/api/s/{exp_token}/download"), "", Body::empty()).await;
        assert_eq!(s, StatusCode::NOT_FOUND);
        assert_eq!(alice.json("GET", &shares_uri, json!({})).await.1.as_array().unwrap().len(), 2);

        // password share
        let (_, locked) = alice.json("POST", &shares_uri, json!({ "password": "s3cret" })).await;
        assert_eq!(locked["has_password"], true);
        let dl = format!("/api/s/{}/download", locked["token"].as_str().unwrap());
        let form = "application/x-www-form-urlencoded";
        let (s, b) = anon.send("GET", &dl, "", Body::empty()).await;
        assert_eq!((s, serde_json::from_slice::<Value>(&b).unwrap()), (StatusCode::FORBIDDEN, json!({ "error": "password required" })));
        let (s, b) = anon.send("POST", &dl, form, Body::from("password=wrong")).await;
        assert_eq!((s, serde_json::from_slice::<Value>(&b).unwrap()), (StatusCode::FORBIDDEN, json!({ "error": "wrong password" })));
        let (s, b) = anon.send("POST", &dl, form, Body::from("password=s3cret")).await;
        assert_eq!((s, b.as_slice()), (StatusCode::OK, &b"hello"[..]));

        // deleting the file removes its shares
        assert_eq!(alice.json("DELETE", &format!("/api/files/{fid}"), json!({})).await.0, StatusCode::NO_CONTENT);
        assert_eq!(anon.json("GET", &format!("/api/s/{token}"), json!({})).await.0, StatusCode::NOT_FOUND);
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM shares").fetch_one(&db).await.unwrap();
        assert_eq!(n, 0);

        db.close().await;
        let _ = std::fs::remove_dir_all(dir);
    }
}
