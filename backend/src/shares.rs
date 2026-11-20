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

use crate::{
    AppError, AppState, Result,
    auth::AuthUser,
    files::{Crumb, Listing, stream_file},
};

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
    is_folder: bool,
    /// Both None for a folder share.
    size_bytes: Option<i64>,
    mime_type: Option<String>,
    expires_at: Option<String>,
    password_required: bool,
}

/// `?preview` serves the file without counting it as a download (image preview on the share page).
#[derive(Deserialize)]
pub struct Opts {
    preview: Option<String>,
    /// Which file to download from a folder share.
    file_id: Option<i64>,
}

/// Lists a shared folder, or `folder_id` somewhere inside it.
#[derive(Deserialize)]
pub struct Browse {
    password: Option<String>,
    folder_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct Unlock {
    password: String,
}

#[derive(sqlx::FromRow)]
struct Shared {
    id: i64,
    file_id: Option<i64>,
    folder_id: Option<i64>,
    name: String,
    mime_type: Option<String>,
    size_bytes: Option<i64>,
    password_hash: Option<String>,
    expires_at: Option<String>,
}

fn forbidden(msg: &str) -> AppError {
    AppError(StatusCode::FORBIDDEN, msg.into())
}

/// Live share by token. Expired, missing, and "its file or folder is in the trash" are all the same 404.
async fn find(app: &AppState, token: &str) -> Result<Shared> {
    sqlx::query_as(
        "SELECT s.id, s.file_id, s.folder_id, COALESCE(f.name, d.name) AS name, f.mime_type, f.size_bytes,
                s.password_hash, s.expires_at
         FROM shares s
         LEFT JOIN files f ON f.id = s.file_id AND f.deleted_at IS NULL
         LEFT JOIN folders d ON d.id = s.folder_id AND d.deleted_at IS NULL
         WHERE s.token = ? AND (f.id IS NOT NULL OR d.id IS NOT NULL)
           AND (s.expires_at IS NULL OR s.expires_at > datetime('now'))",
    )
    .bind(token)
    .fetch_optional(&app.db)
    .await?
    .ok_or_else(AppError::not_found)
}

/// Checks the share's password, if it has one. `given` is None when the request had no way to carry one.
async fn check_password(s: &Shared, given: Option<String>) -> Result<()> {
    let Some(hash) = s.password_hash.clone() else { return Ok(()) };
    let Some(given) = given else { return Err(forbidden("password required")) };
    let ok = tokio::task::spawn_blocking(move || {
        PasswordHash::new(&hash).is_ok_and(|h| Argon2::default().verify_password(given.as_bytes(), &h).is_ok())
    })
    .await
    .map_err(AppError::internal)?;
    if ok { Ok(()) } else { Err(forbidden("wrong password")) }
}

/// Streams the share's file, or `file_id` from a shared folder (404 unless it lives somewhere under that folder).
async fn stream(app: &AppState, s: Shared, file_id: Option<i64>, count: bool) -> Result<Response> {
    let fid = s.file_id.or(file_id).ok_or_else(|| AppError::bad("file_id is required for a folder share"))?;
    let (name, mime, size, path): (String, String, i64, String) = sqlx::query_as(
        "WITH RECURSIVE up(id, parent_id) AS (
            SELECT id, parent_id FROM folders WHERE id = (SELECT folder_id FROM files WHERE id = ?1)
            UNION ALL
            SELECT p.id, p.parent_id FROM folders p JOIN up ON p.id = up.parent_id
         )
         SELECT name, mime_type, size_bytes, storage_path FROM files
         WHERE id = ?1 AND deleted_at IS NULL AND (?2 IS NULL OR EXISTS (SELECT 1 FROM up WHERE id = ?2))",
    )
    .bind(fid)
    .bind(s.folder_id)
    .fetch_optional(&app.db)
    .await?
    .ok_or_else(AppError::not_found)?;
    // No Range here: a ranged share download would be counted once per chunk.
    let res = stream_file(app, &name, mime, size, &path, None).await?;
    if count {
        sqlx::query("UPDATE shares SET download_count = download_count + 1 WHERE id = ?")
            .bind(s.id)
            .execute(&app.db)
            .await?;
    }
    Ok(res)
}

/// Validates the options and inserts a share; `insert` picks the target table (file or folder).
async fn new_share(app: &AppState, uid: i64, insert: &'static str, id: i64, n: NewShare) -> Result<(StatusCode, Json<Share>)> {
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
    let share = sqlx::query_as(insert)
        .bind(uuid::Uuid::new_v4().simple().to_string())
        .bind(hash)
        .bind(n.expires_in_days)
        .bind(id)
        .bind(uid)
        .fetch_optional(&app.db)
        .await?
        .ok_or_else(AppError::not_found)?;
    Ok((StatusCode::CREATED, Json(share)))
}

// datetime('now', NULL) is NULL, so no expiry needs no special case.
pub async fn create(
    State(app): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(file_id): Path<i64>,
    Json(n): Json<NewShare>,
) -> Result<(StatusCode, Json<Share>)> {
    let insert = "INSERT INTO shares (token, file_id, user_id, password_hash, expires_at)
         SELECT ?, id, user_id, ?, datetime('now', '+' || ? || ' days') FROM files
         WHERE id = ? AND user_id = ? AND deleted_at IS NULL
         RETURNING id, token, expires_at, password_hash IS NOT NULL AS has_password, download_count, created_at";
    new_share(&app, uid, insert, file_id, n).await
}

pub async fn create_folder(
    State(app): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(folder_id): Path<i64>,
    Json(n): Json<NewShare>,
) -> Result<(StatusCode, Json<Share>)> {
    let insert = "INSERT INTO shares (token, folder_id, user_id, password_hash, expires_at)
         SELECT ?, id, user_id, ?, datetime('now', '+' || ? || ' days') FROM folders
         WHERE id = ? AND user_id = ? AND deleted_at IS NULL
         RETURNING id, token, expires_at, password_hash IS NOT NULL AS has_password, download_count, created_at";
    new_share(&app, uid, insert, folder_id, n).await
}

async fn shares_of(app: &AppState, uid: i64, select: &'static str, id: i64) -> Result<Json<Vec<Share>>> {
    Ok(Json(sqlx::query_as(select).bind(id).bind(uid).fetch_all(&app.db).await?))
}

pub async fn list(State(app): State<AppState>, AuthUser(uid): AuthUser, Path(file_id): Path<i64>) -> Result<Json<Vec<Share>>> {
    let select = "SELECT id, token, expires_at, password_hash IS NOT NULL AS has_password, download_count, created_at
         FROM shares WHERE file_id = ? AND user_id = ? ORDER BY id DESC";
    shares_of(&app, uid, select, file_id).await
}

pub async fn list_folder(
    State(app): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(folder_id): Path<i64>,
) -> Result<Json<Vec<Share>>> {
    let select = "SELECT id, token, expires_at, password_hash IS NOT NULL AS has_password, download_count, created_at
         FROM shares WHERE folder_id = ? AND user_id = ? ORDER BY id DESC";
    shares_of(&app, uid, select, folder_id).await
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
        is_folder: s.folder_id.is_some(),
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
    check_password(&s, None).await?;
    stream(&app, s, o.file_id, o.preview.is_none()).await
}

pub async fn unlock(
    State(app): State<AppState>,
    Path(token): Path<String>,
    Query(o): Query<Opts>,
    Form(u): Form<Unlock>,
) -> Result<Response> {
    let s = find(&app, &token).await?;
    check_password(&s, Some(u.password)).await?;
    stream(&app, s, o.file_id, true).await
}

/// Public listing of a shared folder. POST so the password stays out of URLs and logs.
pub async fn browse(
    State(app): State<AppState>,
    Path(token): Path<String>,
    Json(b): Json<Browse>,
) -> Result<Json<Listing>> {
    let s = find(&app, &token).await?;
    let root = s.folder_id.ok_or_else(AppError::not_found)?;
    check_password(&s, b.password).await?;
    let target = b.folder_id.unwrap_or(root);
    // Climb from the requested folder and stop at the shared one; a folder outside the share never reaches it.
    let breadcrumbs: Vec<Crumb> = sqlx::query_as(
        "WITH RECURSIVE chain(id, name, parent_id, depth) AS (
            SELECT id, name, parent_id, 0 FROM folders WHERE id = ?1 AND deleted_at IS NULL
            UNION ALL
            SELECT f.id, f.name, f.parent_id, c.depth + 1 FROM folders f JOIN chain c ON f.id = c.parent_id
            WHERE c.id <> ?2
         )
         SELECT id, name FROM chain ORDER BY depth DESC",
    )
    .bind(target)
    .bind(root)
    .fetch_all(&app.db)
    .await?;
    if breadcrumbs.first().map(|c| c.id) != Some(root) {
        return Err(AppError::not_found());
    }
    let folders = sqlx::query_as(
        "SELECT id, name, parent_id, created_at FROM folders
         WHERE parent_id = ? AND deleted_at IS NULL ORDER BY name COLLATE NOCASE",
    )
    .bind(target)
    .fetch_all(&app.db)
    .await?;
    let files = sqlx::query_as(
        "SELECT id, name, mime_type, size_bytes, folder_id, created_at FROM files
         WHERE folder_id = ? AND deleted_at IS NULL ORDER BY name COLLATE NOCASE",
    )
    .bind(target)
    .fetch_all(&app.db)
    .await?;
    Ok(Json(Listing { folders, files, breadcrumbs }))
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
        assert_eq!(meta, json!({ "name": "hello.txt", "is_folder": false, "size_bytes": 5, "mime_type": "text/plain", "expires_at": null, "password_required": false }));
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

        // folder share: browse the subtree, download files in it, nothing outside it
        let docs = folder["id"].as_i64().unwrap();
        let sub = alice.json("POST", "/api/folders", json!({ "name": "Sub", "parent_id": docs })).await.1["id"].as_i64().unwrap();
        let inner = alice.upload(sub, "inner.txt", b"deep").await.1["id"].as_i64().unwrap();
        let other = alice.json("POST", "/api/folders", json!({ "name": "Private" })).await.1["id"].as_i64().unwrap();
        let secret = alice.upload(other, "secret.txt", b"no").await.1["id"].as_i64().unwrap();
        let fshares = format!("/api/folders/{docs}/shares");
        assert_eq!(bob.json("POST", &fshares, json!({})).await.0, StatusCode::NOT_FOUND);
        let (s, fs) = alice.json("POST", &fshares, json!({ "password": "s3cret" })).await;
        assert_eq!(s, StatusCode::CREATED);
        assert_eq!(alice.json("GET", &fshares, json!({})).await.1.as_array().unwrap().len(), 1);
        let base = format!("/api/s/{}", fs["token"].as_str().unwrap());
        let info = anon.json("GET", &base, json!({})).await.1;
        assert_eq!((&info["name"], &info["is_folder"], &info["password_required"]), (&json!("Docs"), &json!(true), &json!(true)));
        let list = format!("{base}/list");
        assert_eq!(anon.json("POST", &list, json!({})).await.0, StatusCode::FORBIDDEN);
        assert_eq!(anon.json("POST", &list, json!({ "password": "nope" })).await.0, StatusCode::FORBIDDEN);
        let l = anon.json("POST", &list, json!({ "password": "s3cret" })).await.1;
        assert_eq!((&l["folders"][0]["name"], &l["files"][0]["name"]), (&json!("Sub"), &json!("hello.txt")));
        assert_eq!(l["breadcrumbs"], json!([{ "id": docs, "name": "Docs" }]));
        let l = anon.json("POST", &list, json!({ "password": "s3cret", "folder_id": sub })).await.1;
        assert_eq!(l["files"][0]["name"], "inner.txt");
        assert_eq!(l["breadcrumbs"], json!([{ "id": docs, "name": "Docs" }, { "id": sub, "name": "Sub" }]));
        assert_eq!(anon.json("POST", &list, json!({ "password": "s3cret", "folder_id": other })).await.0, StatusCode::NOT_FOUND);
        let get = |id: i64| format!("{base}/download?file_id={id}");
        assert_eq!(anon.send("GET", &get(inner), "", Body::empty()).await.0, StatusCode::FORBIDDEN);
        let (s, b) = anon.send("POST", &get(inner), form, Body::from("password=s3cret")).await;
        assert_eq!((s, b.as_slice()), (StatusCode::OK, &b"deep"[..]));
        assert_eq!(anon.send("POST", &get(secret), form, Body::from("password=s3cret")).await.0, StatusCode::NOT_FOUND);
        assert_eq!(anon.send("POST", &format!("{base}/download"), form, Body::from("password=s3cret")).await.0, StatusCode::BAD_REQUEST);
        // a file share can't be pointed at another file, or browsed
        assert_eq!(anon.send("GET", &format!("/api/s/{token}/download?file_id={secret}"), "", Body::empty()).await.1, b"hello");
        assert_eq!(anon.json("POST", &format!("/api/s/{token}/list"), json!({})).await.0, StatusCode::NOT_FOUND);
        // trashing the shared folder kills the link, restoring brings it back
        alice.json("DELETE", &format!("/api/folders/{docs}"), json!({})).await;
        assert_eq!(anon.json("GET", &base, json!({})).await.0, StatusCode::NOT_FOUND);
        alice.json("POST", &format!("/api/trash/folders/{docs}"), json!({})).await;
        assert_eq!(anon.json("GET", &base, json!({})).await.0, StatusCode::OK);
        alice.json("DELETE", &format!("/api/shares/{}", fs["id"]), json!({})).await;

        // trashing the file kills its links; purging it removes the shares
        assert_eq!(alice.json("DELETE", &format!("/api/files/{fid}"), json!({})).await.0, StatusCode::NO_CONTENT);
        assert_eq!(anon.json("GET", &format!("/api/s/{token}"), json!({})).await.0, StatusCode::NOT_FOUND);
        assert_eq!(alice.json("POST", &shares_uri, json!({})).await.0, StatusCode::NOT_FOUND);
        assert_eq!(alice.json("DELETE", &format!("/api/trash/files/{fid}"), json!({})).await.0, StatusCode::NO_CONTENT);
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM shares").fetch_one(&db).await.unwrap();
        assert_eq!(n, 0);

        db.close().await;
        let _ = std::fs::remove_dir_all(dir);
    }
}
