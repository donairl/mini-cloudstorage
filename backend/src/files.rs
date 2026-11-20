use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::Response,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
};
use tokio_util::io::ReaderStream;

use crate::{App, AppError, AppState, Result, auth::AuthUser};

#[derive(Serialize, sqlx::FromRow)]
pub struct Folder {
    id: i64,
    name: String,
    parent_id: Option<i64>,
    created_at: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct FileRow {
    id: i64,
    name: String,
    mime_type: String,
    size_bytes: i64,
    folder_id: Option<i64>,
    created_at: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Crumb {
    pub(crate) id: i64,
    name: String,
}

#[derive(Serialize)]
pub struct Listing {
    pub(crate) folders: Vec<Folder>,
    pub(crate) files: Vec<FileRow>,
    pub(crate) breadcrumbs: Vec<Crumb>,
}

#[derive(Serialize)]
pub struct Storage {
    used_bytes: i64,
    quota_bytes: Option<i64>,
    files: i64,
    images: i64,
    videos: i64,
    /// Share links not yet expired, and how many of those need a password.
    active_shares: i64,
    protected_shares: i64,
    /// Downloads through active share links.
    share_downloads: i64,
}

#[derive(Deserialize)]
pub struct FolderQuery {
    folder_id: Option<i64>,
    /// Name search across all of the user's folders; when set, folder_id is ignored by `list`.
    q: Option<String>,
}

#[derive(Deserialize)]
pub struct NewFolder {
    name: String,
    parent_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct Rename {
    name: String,
}

fn clean_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 255 || name.chars().any(|c| c == '/' || c == '\\' || c.is_control()) {
        return Err(AppError::bad("invalid name"));
    }
    Ok(name.to_string())
}

async fn check_folder(db: &SqlitePool, uid: i64, folder: Option<i64>) -> Result<()> {
    let Some(id) = folder else { return Ok(()) };
    sqlx::query("SELECT 1 FROM folders WHERE id = ? AND user_id = ? AND deleted_at IS NULL")
        .bind(id)
        .bind(uid)
        .fetch_optional(db)
        .await?
        .map(|_| ())
        .ok_or_else(AppError::not_found)
}

async fn remove_files(app: &App, paths: Vec<String>) {
    for p in paths {
        if let Err(e) = tokio::fs::remove_file(app.storage_dir.join(&p)).await
            && e.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!("failed to remove {p}: {e}");
        }
    }
}

fn quota_exceeded() -> AppError {
    AppError(StatusCode::INSUFFICIENT_STORAGE, "Storage quota exceeded".into())
}

/// (used bytes, effective quota); quota None = unlimited. Files in the trash still count until purged.
async fn usage(app: &App, uid: i64) -> Result<(i64, Option<i64>)> {
    sqlx::query_as(
        "SELECT (SELECT COALESCE(SUM(size_bytes), 0) FROM files WHERE user_id = ?1), COALESCE(quota_bytes, ?2)
         FROM users WHERE id = ?1",
    )
    .bind(uid)
    .bind(app.default_quota)
    .fetch_optional(&app.db)
    .await?
    .ok_or_else(AppError::unauthorized)
}

fn not_found_if_zero(res: sqlx::sqlite::SqliteQueryResult) -> Result<StatusCode> {
    match res.rows_affected() {
        0 => Err(AppError::not_found()),
        _ => Ok(StatusCode::NO_CONTENT),
    }
}

/// RFC 5987 percent-encoding for `filename*=UTF-8''...`.
fn encode_filename(name: &str) -> String {
    name.bytes()
        .map(|b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// `%term%` with LIKE wildcards in the term escaped (ESCAPE '\\').
fn like_pattern(term: &str) -> String {
    format!("%{}%", term.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"))
}

pub async fn list(
    State(app): State<AppState>,
    AuthUser(uid): AuthUser,
    Query(q): Query<FolderQuery>,
) -> Result<Json<Listing>> {
    // ponytail: LIKE is case-insensitive for ASCII only and scans every row; switch to FTS5 if that matters.
    let like = q.q.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(like_pattern);
    let breadcrumbs: Vec<Crumb> = match q.folder_id.filter(|_| like.is_none()) {
        None => vec![],
        Some(id) => {
            let crumbs: Vec<Crumb> = sqlx::query_as(
                "WITH RECURSIVE chain(id, name, parent_id, depth) AS (
                    SELECT id, name, parent_id, 0 FROM folders WHERE id = ? AND user_id = ? AND deleted_at IS NULL
                    UNION ALL
                    SELECT f.id, f.name, f.parent_id, c.depth + 1 FROM folders f JOIN chain c ON f.id = c.parent_id
                 )
                 SELECT id, name FROM chain ORDER BY depth DESC",
            )
            .bind(id)
            .bind(uid)
            .fetch_all(&app.db)
            .await?;
            if crumbs.is_empty() {
                return Err(AppError::not_found());
            }
            crumbs
        }
    };
    let folders = sqlx::query_as(
        "SELECT id, name, parent_id, created_at FROM folders
         WHERE user_id = ?1 AND deleted_at IS NULL AND CASE WHEN ?3 IS NULL THEN parent_id IS ?2 ELSE name LIKE ?3 ESCAPE '\\' END
         ORDER BY name COLLATE NOCASE",
    )
    .bind(uid)
    .bind(q.folder_id)
    .bind(&like)
    .fetch_all(&app.db)
    .await?;
    let files = sqlx::query_as(
        "SELECT id, name, mime_type, size_bytes, folder_id, created_at FROM files
         WHERE user_id = ?1 AND deleted_at IS NULL AND CASE WHEN ?3 IS NULL THEN folder_id IS ?2 ELSE name LIKE ?3 ESCAPE '\\' END
         ORDER BY name COLLATE NOCASE",
    )
    .bind(uid)
    .bind(q.folder_id)
    .bind(&like)
    .fetch_all(&app.db)
    .await?;
    Ok(Json(Listing { folders, files, breadcrumbs }))
}

pub async fn storage(State(app): State<AppState>, AuthUser(uid): AuthUser) -> Result<Json<Storage>> {
    let (used_bytes, quota_bytes) = usage(&app, uid).await?;
    let (files, images, videos, active_shares, protected_shares, share_downloads) = sqlx::query_as(
        "SELECT COUNT(*),
                COALESCE(SUM(mime_type LIKE 'image/%'), 0),
                COALESCE(SUM(mime_type LIKE 'video/%'), 0),
                (SELECT COUNT(*) FROM shares WHERE user_id = ?1 AND (file_id IN (SELECT id FROM files WHERE deleted_at IS NULL)
                        OR folder_id IN (SELECT id FROM folders WHERE deleted_at IS NULL)) AND (expires_at IS NULL OR expires_at > datetime('now'))),
                (SELECT COUNT(*) FROM shares WHERE user_id = ?1 AND (file_id IN (SELECT id FROM files WHERE deleted_at IS NULL)
                        OR folder_id IN (SELECT id FROM folders WHERE deleted_at IS NULL)) AND (expires_at IS NULL OR expires_at > datetime('now'))
                    AND password_hash IS NOT NULL),
                (SELECT COALESCE(SUM(download_count), 0) FROM shares WHERE user_id = ?1 AND (file_id IN (SELECT id FROM files WHERE deleted_at IS NULL)
                        OR folder_id IN (SELECT id FROM folders WHERE deleted_at IS NULL))
                    AND (expires_at IS NULL OR expires_at > datetime('now')))
         FROM files WHERE user_id = ?1 AND deleted_at IS NULL",
    )
    .bind(uid)
    .fetch_one(&app.db)
    .await?;
    Ok(Json(Storage { used_bytes, quota_bytes, files, images, videos, active_shares, protected_shares, share_downloads }))
}

pub async fn upload(
    State(app): State<AppState>,
    AuthUser(uid): AuthUser,
    Query(q): Query<FolderQuery>,
    mut mp: Multipart,
) -> Result<(StatusCode, Json<FileRow>)> {
    check_folder(&app.db, uid, q.folder_id).await?;
    let (used, quota) = usage(&app, uid).await?;
    let remaining = quota.map(|q| q - used);
    let mut field = loop {
        match mp.next_field().await? {
            Some(f) if f.name() == Some("file") => break f,
            Some(_) => continue,
            None => return Err(AppError::bad("missing file field")),
        }
    };
    let raw = field.file_name().unwrap_or_default();
    let name = clean_name(raw.rsplit(['/', '\\']).next().unwrap_or_default())?;
    let mime = field.content_type().unwrap_or("application/octet-stream").to_string();
    let storage_path = uuid::Uuid::new_v4().to_string();
    let path = app.storage_dir.join(&storage_path);

    let res: Result<FileRow> = async {
        let mut out = File::create(&path).await?;
        let mut size: i64 = 0;
        while let Some(chunk) = field.chunk().await? {
            size += chunk.len() as i64;
            if remaining.is_some_and(|r| size > r) {
                return Err(quota_exceeded());
            }
            out.write_all(&chunk).await?;
        }
        out.flush().await?;
        // Re-checked inside the INSERT: a single write statement is atomic, so concurrent uploads can't both fit.
        sqlx::query_as(
            "INSERT INTO files (name, mime_type, size_bytes, storage_path, folder_id, user_id)
             SELECT ?1, ?2, ?3, ?4, ?5, ?6 FROM users
             WHERE id = ?6 AND (COALESCE(quota_bytes, ?7) IS NULL
                OR (SELECT COALESCE(SUM(size_bytes), 0) FROM files WHERE user_id = ?6) + ?3 <= COALESCE(quota_bytes, ?7))
             RETURNING id, name, mime_type, size_bytes, folder_id, created_at",
        )
        .bind(name)
        .bind(mime)
        .bind(size)
        .bind(&storage_path)
        .bind(q.folder_id)
        .bind(uid)
        .bind(app.default_quota)
        .fetch_optional(&app.db)
        .await?
        .ok_or_else(quota_exceeded)
    }
    .await;
    if res.is_err() {
        let _ = tokio::fs::remove_file(&path).await;
    }
    res.map(|f| (StatusCode::CREATED, Json(f)))
}

pub async fn download(
    State(app): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response> {
    let (name, mime, size, storage_path): (String, String, i64, String) =
        sqlx::query_as("SELECT name, mime_type, size_bytes, storage_path FROM files WHERE id = ? AND user_id = ? AND deleted_at IS NULL")
            .bind(id)
            .bind(uid)
            .fetch_optional(&app.db)
            .await?
            .ok_or_else(AppError::not_found)?;
    let range = headers.get(header::RANGE).and_then(|v| v.to_str().ok());
    stream_file(&app, &name, mime, size, &storage_path, range).await
}

/// Inclusive byte span for a `Range` header value. `Ok(None)` = serve the whole file (no usable range:
/// malformed, other unit, or multi-range, which we don't support); `Err` = range is outside the file.
fn parse_range(range: &str, size: u64) -> std::result::Result<Option<(u64, u64)>, ()> {
    let Some((start, end)) = range.trim().strip_prefix("bytes=").and_then(|r| r.split_once('-')) else {
        return Ok(None);
    };
    let span = match (start.trim().parse::<u64>(), end.trim().parse::<u64>()) {
        (Ok(s), Ok(e)) => (s, e.min(size.saturating_sub(1))),
        (Ok(s), Err(_)) if end.trim().is_empty() => (s, size.saturating_sub(1)),
        // suffix form: the last n bytes
        (Err(_), Ok(n)) if start.trim().is_empty() => (size.saturating_sub(n), size.saturating_sub(1)),
        _ => return Ok(None),
    };
    if span.0 >= size || span.0 > span.1 { Err(()) } else { Ok(Some(span)) }
}

/// Streams a stored file as an attachment download; `range` is the request's `Range` header, if any.
pub(crate) async fn stream_file(
    app: &App,
    name: &str,
    mime: String,
    size: i64,
    storage_path: &str,
    range: Option<&str>,
) -> Result<Response> {
    let size = size as u64;
    let span = range
        .map_or(Ok(None), |r| parse_range(r, size))
        .map_err(|_| AppError(StatusCode::RANGE_NOT_SATISFIABLE, "range not satisfiable".into()))?;
    let (start, end) = span.unwrap_or((0, size.saturating_sub(1)));
    let len = if size == 0 { 0 } else { end - start + 1 };
    let mut file = File::open(app.storage_dir.join(storage_path)).await?;
    file.seek(std::io::SeekFrom::Start(start)).await?;

    let mut res = Response::builder()
        .status(if span.is_some() { StatusCode::PARTIAL_CONTENT } else { StatusCode::OK })
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, len)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename*=UTF-8''{}", encode_filename(name)))
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff");
    if span.is_some() {
        res = res.header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}"));
    }
    res.body(Body::from_stream(ReaderStream::new(file.take(len)))).map_err(|e| AppError::bad(&e.to_string()))
}

pub async fn rename_file(
    State(app): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<i64>,
    Json(r): Json<Rename>,
) -> Result<StatusCode> {
    let name = clean_name(&r.name)?;
    let res = sqlx::query("UPDATE files SET name = ? WHERE id = ? AND user_id = ? AND deleted_at IS NULL")
        .bind(name)
        .bind(id)
        .bind(uid)
        .execute(&app.db)
        .await?;
    not_found_if_zero(res)
}

// The trash lists, and restore accepts, only trashed items whose parent is still live ("parent_id IS NULL OR
// parent_id IN (live folders)" below). Anything deeper comes and goes with its trashed ancestor.

#[derive(Serialize)]
pub struct Trash {
    folders: Vec<Folder>,
    files: Vec<FileRow>,
}

pub async fn trash(State(app): State<AppState>, AuthUser(uid): AuthUser) -> Result<Json<Trash>> {
    let folders = sqlx::query_as(
        "SELECT id, name, parent_id, created_at FROM folders
         WHERE user_id = ? AND deleted_at IS NOT NULL
           AND (parent_id IS NULL OR parent_id IN (SELECT id FROM folders WHERE deleted_at IS NULL))
         ORDER BY deleted_at DESC, id DESC",
    )
    .bind(uid)
    .fetch_all(&app.db)
    .await?;
    let files = sqlx::query_as(
        "SELECT id, name, mime_type, size_bytes, folder_id, created_at FROM files
         WHERE user_id = ? AND deleted_at IS NOT NULL
           AND (folder_id IS NULL OR folder_id IN (SELECT id FROM folders WHERE deleted_at IS NULL))
         ORDER BY deleted_at DESC, id DESC",
    )
    .bind(uid)
    .fetch_all(&app.db)
    .await?;
    Ok(Json(Trash { folders, files }))
}

/// Moves a file to the trash.
pub async fn trash_file(State(app): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<i64>) -> Result<StatusCode> {
    let res = sqlx::query(
        "UPDATE files SET deleted_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
         WHERE id = ? AND user_id = ? AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(uid)
    .execute(&app.db)
    .await?;
    not_found_if_zero(res)
}

pub async fn restore_file(State(app): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<i64>) -> Result<StatusCode> {
    let res = sqlx::query(
        "UPDATE files SET deleted_at = NULL WHERE id = ? AND user_id = ? AND deleted_at IS NOT NULL
           AND (folder_id IS NULL OR folder_id IN (SELECT id FROM folders WHERE deleted_at IS NULL))",
    )
    .bind(id)
    .bind(uid)
    .execute(&app.db)
    .await?;
    not_found_if_zero(res)
}

/// Sets deleted_at to `to` on every row of the folder's subtree whose deleted_at is `from`.
/// 404 when the folder itself isn't in the `from` state.
async fn stamp_subtree(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
    uid: i64,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<StatusCode> {
    macro_rules! in_subtree {
        ($update:literal) => {
            concat!(
                "WITH RECURSIVE sub(id) AS (
                    SELECT id FROM folders WHERE id = ?1 AND user_id = ?2 AND deleted_at IS ?3
                    UNION ALL
                    SELECT f.id FROM folders f JOIN sub ON f.parent_id = sub.id
                 ) UPDATE ",
                $update,
                " IN (SELECT id FROM sub) AND deleted_at IS ?3"
            )
        };
    }
    // Files first: the folder update changes the root row the CTE starts from.
    let mut res = None;
    for sql in [in_subtree!("files SET deleted_at = ?4 WHERE folder_id"), in_subtree!("folders SET deleted_at = ?4 WHERE id")] {
        res = Some(sqlx::query(sql).bind(id).bind(uid).bind(from).bind(to).execute(&mut *conn).await?);
    }
    not_found_if_zero(res.expect("loop ran"))
}

/// Moves a folder and everything in it to the trash.
pub async fn trash_folder(State(app): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<i64>) -> Result<StatusCode> {
    let mut tx = app.db.begin_with("BEGIN IMMEDIATE").await?;
    let stamp: String = sqlx::query_scalar("SELECT strftime('%Y-%m-%d %H:%M:%f', 'now')").fetch_one(&mut *tx).await?;
    let status = stamp_subtree(&mut tx, id, uid, None, Some(&stamp)).await?;
    tx.commit().await?;
    Ok(status)
}

pub async fn restore_folder(State(app): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<i64>) -> Result<StatusCode> {
    let mut tx = app.db.begin_with("BEGIN IMMEDIATE").await?;
    let stamp: String = sqlx::query_scalar(
        "SELECT deleted_at FROM folders WHERE id = ? AND user_id = ? AND deleted_at IS NOT NULL
           AND (parent_id IS NULL OR parent_id IN (SELECT id FROM folders WHERE deleted_at IS NULL))",
    )
    .bind(id)
    .bind(uid)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(AppError::not_found)?;
    let status = stamp_subtree(&mut tx, id, uid, Some(&stamp), None).await?;
    tx.commit().await?;
    Ok(status)
}

/// Permanently deletes everything in the user's trash.
pub async fn empty_trash(State(app): State<AppState>, AuthUser(uid): AuthUser) -> Result<StatusCode> {
    let mut tx = app.db.begin_with("BEGIN IMMEDIATE").await?;
    let paths: Vec<String> =
        sqlx::query_scalar("DELETE FROM files WHERE user_id = ? AND deleted_at IS NOT NULL RETURNING storage_path")
            .bind(uid)
            .fetch_all(&mut *tx)
            .await?;
    // Everything under a trashed folder is trashed too, so the cascade removes nothing live.
    sqlx::query("DELETE FROM folders WHERE user_id = ? AND deleted_at IS NOT NULL").bind(uid).execute(&mut *tx).await?;
    tx.commit().await?;
    remove_files(&app, paths).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Permanently deletes a file that is in the trash.
pub async fn purge_file(
    State(app): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode> {
    let path: String = sqlx::query_scalar(
        "DELETE FROM files WHERE id = ? AND user_id = ? AND deleted_at IS NOT NULL RETURNING storage_path",
    )
        .bind(id)
        .bind(uid)
        .fetch_optional(&app.db)
        .await?
        .ok_or_else(AppError::not_found)?;
    remove_files(&app, vec![path]).await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_folder(
    State(app): State<AppState>,
    AuthUser(uid): AuthUser,
    Json(f): Json<NewFolder>,
) -> Result<(StatusCode, Json<Folder>)> {
    let name = clean_name(&f.name)?;
    check_folder(&app.db, uid, f.parent_id).await?;
    let folder = sqlx::query_as(
        "INSERT INTO folders (name, parent_id, user_id) VALUES (?, ?, ?)
         RETURNING id, name, parent_id, created_at",
    )
    .bind(name)
    .bind(f.parent_id)
    .bind(uid)
    .fetch_one(&app.db)
    .await?;
    Ok((StatusCode::CREATED, Json(folder)))
}

pub async fn rename_folder(
    State(app): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<i64>,
    Json(r): Json<Rename>,
) -> Result<StatusCode> {
    let name = clean_name(&r.name)?;
    let res = sqlx::query("UPDATE folders SET name = ? WHERE id = ? AND user_id = ? AND deleted_at IS NULL")
        .bind(name)
        .bind(id)
        .bind(uid)
        .execute(&app.db)
        .await?;
    not_found_if_zero(res)
}

/// Permanently deletes a folder that is in the trash, with everything inside.
pub async fn purge_folder(
    State(app): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode> {
    // IMMEDIATE takes the write lock up front: a deferred tx upgrading after the SELECT fails with BUSY_SNAPSHOT under WAL.
    let mut tx = app.db.begin_with("BEGIN IMMEDIATE").await?;
    let paths: Vec<String> = sqlx::query_scalar(
        "WITH RECURSIVE sub(id) AS (
            SELECT id FROM folders WHERE id = ? AND user_id = ? AND deleted_at IS NOT NULL
            UNION ALL
            SELECT f.id FROM folders f JOIN sub ON f.parent_id = sub.id
         )
         SELECT storage_path FROM files WHERE folder_id IN (SELECT id FROM sub)",
    )
    .bind(id)
    .bind(uid)
    .fetch_all(&mut *tx)
    .await?;
    let res = sqlx::query("DELETE FROM folders WHERE id = ? AND user_id = ? AND deleted_at IS NOT NULL")
        .bind(id)
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    let status = not_found_if_zero(res)?;
    tx.commit().await?;
    remove_files(&app, paths).await;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use crate::tests::{Client, test_app, user};
    use axum::http::StatusCode;
    use serde_json::{Value, json};

    async fn folder(c: &mut Client) -> i64 {
        c.json("POST", "/api/folders", json!({ "name": "f" })).await.1["id"].as_i64().unwrap()
    }

    // Just the quota part; counts are covered by `stats`.
    async fn storage(c: &mut Client) -> Value {
        let s = c.json("GET", "/api/storage", json!({})).await.1;
        json!({ "used_bytes": s["used_bytes"], "quota_bytes": s["quota_bytes"] })
    }

    #[tokio::test]
    async fn quota() {
        let (app, dir, storage_dir) = test_app(1024 * 1024, Some(10)).await;
        let files = || std::fs::read_dir(&storage_dir).unwrap().count();
        let mut alice = user(&app, "alice").await;
        let f = folder(&mut alice).await;

        assert_eq!(storage(&mut alice).await, json!({ "used_bytes": 0, "quota_bytes": 10 }));
        let (s, file) = alice.upload(f, "a.txt", b"123456").await;
        assert_eq!(s, StatusCode::CREATED);
        let (s, body) = alice.upload(f, "b.txt", b"123456").await;
        assert_eq!(s, StatusCode::INSUFFICIENT_STORAGE);
        assert_eq!(body["error"], "Storage quota exceeded");
        assert_eq!(files(), 1, "rejected upload left on disk");
        assert_eq!(storage(&mut alice).await, json!({ "used_bytes": 6, "quota_bytes": 10 }));

        // exactly fills the quota
        assert_eq!(alice.upload(f, "c.txt", b"1234").await.0, StatusCode::CREATED);
        assert_eq!(alice.upload(f, "d.txt", b"1").await.0, StatusCode::INSUFFICIENT_STORAGE);

        let id = file["id"].as_i64().unwrap();
        alice.json("DELETE", &format!("/api/files/{id}"), json!({})).await;
        assert_eq!(storage(&mut alice).await["used_bytes"], 10, "trash still counts toward the quota");
        alice.json("DELETE", &format!("/api/trash/files/{id}"), json!({})).await;
        assert_eq!(storage(&mut alice).await["used_bytes"], 4);

        // concurrent uploads: the conditional INSERT lets only one fit
        let mut a2 = Client { app: alice.app.clone(), cookie: alice.cookie.clone() };
        let (r1, r2) = tokio::join!(alice.upload(f, "e.txt", b"12345"), a2.upload(f, "g.txt", b"12345"));
        let mut codes = [r1.0, r2.0];
        codes.sort();
        assert_eq!(codes, [StatusCode::CREATED, StatusCode::INSUFFICIENT_STORAGE]);
        assert_eq!(storage(&mut alice).await["used_bytes"], 9);
        assert_eq!(files(), 2);

        // per-user override beats the default
        let mut bob = user(&app, "bob").await;
        let bf = folder(&mut bob).await;
        let db = sqlx::SqlitePool::connect(&format!("sqlite://{}", dir.join("app.db").display())).await.unwrap();
        sqlx::query("UPDATE users SET quota_bytes = 20 WHERE username = 'bob'").execute(&db).await.unwrap();
        assert_eq!(storage(&mut bob).await, json!({ "used_bytes": 0, "quota_bytes": 20 }));
        assert_eq!(bob.upload(bf, "big.txt", &[0u8; 15]).await.0, StatusCode::CREATED);
        db.close().await;

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn range_parsing() {
        use super::parse_range;
        assert_eq!(parse_range("bytes=1-3", 5), Ok(Some((1, 3))));
        assert_eq!(parse_range("bytes=2-", 5), Ok(Some((2, 4))));
        assert_eq!(parse_range("bytes=-2", 5), Ok(Some((3, 4))));
        assert_eq!(parse_range("bytes=-99", 5), Ok(Some((0, 4))));
        assert_eq!(parse_range("bytes=3-99", 5), Ok(Some((3, 4))), "end is clamped");
        assert_eq!(parse_range("bytes=5-", 5), Err(()));
        assert_eq!(parse_range("bytes=3-1", 5), Err(()));
        assert_eq!(parse_range("bytes=-0", 5), Err(()));
        assert_eq!(parse_range("bytes=0-", 0), Err(()));
        for whole in ["bytes=0-1,3-4", "items=0-1", "bytes=a-b", "bytes=", "bytes=-"] {
            assert_eq!(parse_range(whole, 5), Ok(None), "{whole}");
        }
    }

    #[tokio::test]
    async fn range_download() {
        use axum::{body::Body, http::Request};
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        let (app, dir, _) = test_app(1024 * 1024, None).await;
        let mut alice = user(&app, "alice").await;
        let f = folder(&mut alice).await;
        let id = alice.upload(f, "a.txt", b"hello").await.1["id"].as_i64().unwrap();
        let get = async |range: &str| {
            let req = Request::get(format!("/api/files/download/{id}"))
                .header("cookie", alice.cookie.as_deref().unwrap())
                .header("range", range)
                .body(Body::empty())
                .unwrap();
            let res = app.clone().oneshot(req).await.unwrap();
            let h = |k: &str| res.headers().get(k).map(|v| v.to_str().unwrap().to_string());
            let (status, crange, clen) = (res.status(), h("content-range"), h("content-length"));
            (status, crange, clen, res.into_body().collect().await.unwrap().to_bytes().to_vec())
        };
        assert_eq!(
            get("bytes=1-3").await,
            (StatusCode::PARTIAL_CONTENT, Some("bytes 1-3/5".into()), Some("3".into()), b"ell".to_vec())
        );
        assert_eq!(get("bytes=-2").await.3, b"lo");
        assert_eq!(get("bytes=9-").await.0, StatusCode::RANGE_NOT_SATISFIABLE);
        // unsupported forms fall back to the whole file
        assert_eq!(get("bytes=0-0,2-2").await, (StatusCode::OK, None, Some("5".into()), b"hello".to_vec()));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn trash() {
        let (app, dir, storage_dir) = test_app(1024 * 1024, None).await;
        let disk = || std::fs::read_dir(&storage_dir).unwrap().count();
        let mut alice = user(&app, "alice").await;
        let mut bob = user(&app, "bob").await;
        let names = |v: &Value| v.as_array().unwrap().iter().map(|x| x["name"].as_str().unwrap().to_string()).collect::<Vec<_>>();
        let f = folder(&mut alice).await;
        let sub = alice.json("POST", "/api/folders", json!({ "name": "sub", "parent_id": f })).await.1["id"].as_i64().unwrap();
        let a = alice.upload(f, "a.txt", b"aa").await.1["id"].as_i64().unwrap();
        alice.upload(sub, "b.txt", b"bb").await;
        let token = alice.json("POST", &format!("/api/files/{a}/shares"), json!({})).await.1["token"].as_str().unwrap().to_string();
        let (in_f, in_sub) = (format!("/api/files?folder_id={f}"), format!("/api/files?folder_id={sub}"));

        // a trashed file disappears everywhere but the trash, and keeps its bytes
        assert_eq!(alice.json("DELETE", &format!("/api/files/{a}"), json!({})).await.0, StatusCode::NO_CONTENT);
        assert_eq!(alice.json("DELETE", &format!("/api/files/{a}"), json!({})).await.0, StatusCode::NOT_FOUND);
        assert_eq!(alice.json("GET", &in_f, json!({})).await.1["files"], json!([]));
        assert_eq!(alice.json("GET", "/api/files?q=a.txt", json!({})).await.1["files"], json!([]));
        assert_eq!(alice.json("GET", &format!("/api/files/download/{a}"), json!({})).await.0, StatusCode::NOT_FOUND);
        assert_eq!(alice.json("GET", &format!("/api/s/{token}"), json!({})).await.0, StatusCode::NOT_FOUND);
        assert_eq!(names(&alice.json("GET", "/api/trash", json!({})).await.1["files"]), ["a.txt"]);
        let s = alice.json("GET", "/api/storage", json!({})).await.1;
        assert_eq!((&s["used_bytes"], &s["files"], &s["active_shares"]), (&json!(4), &json!(1), &json!(0)));

        // trashing the folder takes the subtree; only the folder itself is listed
        assert_eq!(alice.json("DELETE", &format!("/api/folders/{f}"), json!({})).await.0, StatusCode::NO_CONTENT);
        assert_eq!(alice.json("GET", "/api/files", json!({})).await.1["folders"], json!([]));
        assert_eq!(alice.json("GET", &in_sub, json!({})).await.0, StatusCode::NOT_FOUND);
        assert_eq!(alice.json("GET", "/api/files?q=b.txt", json!({})).await.1["files"], json!([]));
        assert_eq!(alice.upload(f, "x.txt", b"x").await.0, StatusCode::NOT_FOUND);
        let t = alice.json("GET", "/api/trash", json!({})).await.1;
        assert_eq!((names(&t["folders"]), names(&t["files"])), (vec!["f".to_string()], vec![]));
        // nothing inside a trashed folder can be restored on its own
        assert_eq!(alice.json("POST", &format!("/api/trash/files/{a}"), json!({})).await.0, StatusCode::NOT_FOUND);
        assert_eq!(alice.json("POST", &format!("/api/trash/folders/{sub}"), json!({})).await.0, StatusCode::NOT_FOUND);
        // other users can't touch it
        assert_eq!(bob.json("POST", &format!("/api/trash/folders/{f}"), json!({})).await.0, StatusCode::NOT_FOUND);
        assert_eq!(bob.json("DELETE", &format!("/api/trash/folders/{f}"), json!({})).await.0, StatusCode::NOT_FOUND);
        assert_eq!(bob.json("DELETE", "/api/trash", json!({})).await.0, StatusCode::NO_CONTENT);
        assert_eq!(disk(), 2);

        // restoring the folder brings back what went with it; the file trashed earlier stays in the trash
        assert_eq!(alice.json("POST", &format!("/api/trash/folders/{f}"), json!({})).await.0, StatusCode::NO_CONTENT);
        let l = alice.json("GET", &in_f, json!({})).await.1;
        assert_eq!((names(&l["folders"]), names(&l["files"])), (vec!["sub".to_string()], vec![]));
        assert_eq!(names(&alice.json("GET", &in_sub, json!({})).await.1["files"]), ["b.txt"]);
        assert_eq!(names(&alice.json("GET", "/api/trash", json!({})).await.1["files"]), ["a.txt"]);
        assert_eq!(alice.json("POST", &format!("/api/trash/files/{a}"), json!({})).await.0, StatusCode::NO_CONTENT);
        assert_eq!(names(&alice.json("GET", &in_f, json!({})).await.1["files"]), ["a.txt"]);
        assert_eq!(alice.json("GET", &format!("/api/s/{token}"), json!({})).await.0, StatusCode::OK);

        // purge only works on trashed items, and removes the bytes
        assert_eq!(alice.json("DELETE", &format!("/api/trash/files/{a}"), json!({})).await.0, StatusCode::NOT_FOUND);
        assert_eq!(alice.json("DELETE", &format!("/api/trash/folders/{f}"), json!({})).await.0, StatusCode::NOT_FOUND);
        alice.json("DELETE", &format!("/api/files/{a}"), json!({})).await;
        assert_eq!(alice.json("DELETE", &format!("/api/trash/files/{a}"), json!({})).await.0, StatusCode::NO_CONTENT);
        assert_eq!(disk(), 1);

        // empty trash
        let c = alice.upload(f, "c.txt", b"c").await.1["id"].as_i64().unwrap();
        alice.json("DELETE", &format!("/api/files/{c}"), json!({})).await;
        alice.json("DELETE", &format!("/api/folders/{sub}"), json!({})).await;
        assert_eq!(alice.json("DELETE", "/api/trash", json!({})).await.0, StatusCode::NO_CONTENT);
        assert_eq!(disk(), 0);
        let t = alice.json("GET", "/api/trash", json!({})).await.1;
        assert_eq!((&t["folders"], &t["files"]), (&json!([]), &json!([])));
        assert_eq!(names(&alice.json("GET", "/api/files", json!({})).await.1["folders"]), ["f"]);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn search() {
        let (app, dir, _) = test_app(1024 * 1024, None).await;
        let mut alice = user(&app, "alice").await;
        let f = folder(&mut alice).await;
        alice.json("POST", "/api/folders", json!({ "name": "Reports", "parent_id": f })).await;
        alice.upload(f, "annual report.txt", b"x").await;
        alice.upload(f, "100%.txt", b"x").await;
        alice.upload(f, "a_b.txt", b"x").await;
        let names = |v: &Value| v.as_array().unwrap().iter().map(|x| x["name"].as_str().unwrap().to_string()).collect::<Vec<_>>();

        // matches nested files and folders from the root, case-insensitively
        let r = alice.json("GET", "/api/files?q=REPORT", json!({})).await.1;
        assert_eq!(names(&r["folders"]), ["Reports"]);
        assert_eq!(names(&r["files"]), ["annual report.txt"]);
        assert_eq!(r["breadcrumbs"], json!([]));
        // wildcards are literal
        assert_eq!(names(&alice.json("GET", "/api/files?q=%25", json!({})).await.1["files"]), ["100%.txt"]);
        assert_eq!(names(&alice.json("GET", "/api/files?q=_", json!({})).await.1["files"]), ["a_b.txt"]);
        // blank query falls back to the normal listing
        assert_eq!(names(&alice.json("GET", "/api/files?q=%20", json!({})).await.1["folders"]), ["f"]);
        // other users' files never match
        let mut bob = user(&app, "bob").await;
        let r = bob.json("GET", "/api/files?q=report", json!({})).await.1;
        assert_eq!((&r["folders"], &r["files"]), (&json!([]), &json!([])));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn unlimited() {
        let (app, dir, _) = test_app(1024 * 1024, None).await;
        let mut alice = user(&app, "alice").await;
        let f = folder(&mut alice).await;
        assert_eq!(alice.upload(f, "a.txt", &[0u8; 1000]).await.0, StatusCode::CREATED);
        assert_eq!(storage(&mut alice).await, json!({ "used_bytes": 1000, "quota_bytes": null }));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn stats() {
        let (app, dir, _) = test_app(1024 * 1024, None).await;
        let mut alice = user(&app, "alice").await;
        let f = folder(&mut alice).await;
        let (_, txt) = alice.upload(f, "a.txt", b"hi").await;
        let png = b"--X\r\nContent-Disposition: form-data; name=\"file\"; filename=\"p.png\"\r\nContent-Type: image/png\r\n\r\npng\r\n--X--\r\n";
        let uri = format!("/api/files/upload?folder_id={f}");
        alice.send("POST", &uri, "multipart/form-data; boundary=X", axum::body::Body::from(&png[..])).await;
        let shares = format!("/api/files/{}/shares", txt["id"]);
        alice.json("POST", &shares, json!({ "password": "secret" })).await;
        let token = alice.json("POST", &shares, json!({})).await.1["token"].as_str().unwrap().to_string();
        let mut anon = Client { app: app.clone(), cookie: None };
        let dl = format!("/api/s/{token}/download");
        for uri in [&dl, &dl, &format!("{dl}?preview")] {
            assert_eq!(anon.send("GET", uri, "", axum::body::Body::empty()).await.0, StatusCode::OK);
        }
        let listed = alice.json("GET", &shares, json!({})).await.1;
        assert_eq!(listed[0]["download_count"], 2, "preview must not count");

        let s = alice.json("GET", "/api/storage", json!({})).await.1;
        assert_eq!(
            (&s["files"], &s["images"], &s["videos"], &s["active_shares"], &s["protected_shares"]),
            (&json!(2), &json!(1), &json!(0), &json!(2), &json!(1))
        );
        assert_eq!(s["used_bytes"], 5);
        assert_eq!(s["share_downloads"], 2);
        let _ = std::fs::remove_dir_all(dir);
    }
}
