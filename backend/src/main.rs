mod auth;
mod files;
mod shares;

use std::{path::PathBuf, str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, multipart::MultipartError},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool};
use tower_http::services::{ServeDir, ServeFile};

pub struct App {
    pub db: SqlitePool,
    pub storage_dir: PathBuf,
    pub jwt_secret: String,
    pub max_upload: usize,
    pub cookie_secure: bool,
    /// Per-user storage limit when users.quota_bytes is NULL; None = unlimited.
    pub default_quota: Option<i64>,
}

pub type AppState = Arc<App>;

pub struct AppError(pub StatusCode, pub String);

pub type Result<T> = std::result::Result<T, AppError>;

impl AppError {
    pub fn bad(msg: &str) -> Self {
        Self(StatusCode::BAD_REQUEST, msg.into())
    }
    pub fn not_found() -> Self {
        Self(StatusCode::NOT_FOUND, "not found".into())
    }
    pub fn unauthorized() -> Self {
        Self(StatusCode::UNAUTHORIZED, "unauthorized".into())
    }
    fn internal(e: impl std::fmt::Display) -> Self {
        tracing::error!("{e}");
        Self(StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self::internal(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::internal(e)
    }
}

impl From<MultipartError> for AppError {
    fn from(e: MultipartError) -> Self {
        Self(e.status(), e.body_text())
    }
}

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::me))
        .route("/files", get(files::list))
        .route("/storage", get(files::storage))
        .route(
            "/files/upload",
            post(files::upload).layer(DefaultBodyLimit::max(state.max_upload)),
        )
        .route("/files/download/{id}", get(files::download))
        .route("/files/{id}", patch(files::rename_file).delete(files::trash_file))
        .route("/folders", post(files::create_folder))
        .route("/folders/{id}", patch(files::rename_folder).delete(files::trash_folder))
        .route("/trash", get(files::trash).delete(files::empty_trash))
        .route("/trash/files/{id}", post(files::restore_file).delete(files::purge_file))
        .route("/trash/folders/{id}", post(files::restore_folder).delete(files::purge_folder))
        .route("/files/{id}/shares", get(shares::list).post(shares::create))
        .route("/shares/{id}", delete(shares::revoke))
        .route("/s/{token}", get(shares::info))
        .route("/folders/{id}/shares", get(shares::list_folder).post(shares::create_folder))
        .route("/s/{token}/download", get(shares::download).post(shares::unlock))
        .route("/s/{token}/list", post(shares::browse))
        .route("/s/{token}/zip", get(shares::zip).post(shares::unlock_zip))
        .route("/folders/{id}/zip", get(files::download_folder))
        .fallback(|| async { AppError::not_found() });
    Router::new().nest("/api", api).with_state(state)
}

async fn connect_db(url: &str) -> std::result::Result<SqlitePool, Box<dyn std::error::Error>> {
    let opts = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);
    let db = SqlitePool::connect_with(opts).await?;
    sqlx::migrate!().run(&db).await?;
    Ok(db)
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    let env =|k: &str, default: &str| std::env::var(k).unwrap_or_else(|_| default.into());

    let jwt_secret = std::env::var("JWT_SECRET").map_err(|_| "JWT_SECRET must be set")?;
    let storage_dir = PathBuf::from(env("STORAGE_DIR", "data/files"));
    tokio::fs::create_dir_all(&storage_dir).await?;
    let db = connect_db(&env("DATABASE_URL", "sqlite://data/app.db")).await?;

    let state = Arc::new(App {
        db,
        storage_dir,
        jwt_secret,
        max_upload: env("MAX_UPLOAD_MB", "100").parse::<usize>()? * 1024 * 1024,
        cookie_secure: env("COOKIE_SECURE", "0") == "1",
        default_quota: Some(env("QUOTA_MB", "0").parse::<i64>()? * 1024 * 1024).filter(|&q| q > 0),
    });

    let static_dir = env("STATIC_DIR", "../frontend/dist");
    let spa = ServeDir::new(&static_dir).fallback(ServeFile::new(format!("{static_dir}/index.html")));
    let app = router(state).fallback_service(spa);

    let addr = format!("0.0.0.0:{}", env("PORT", "3000"));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use serde_json::{Value, json};
    use tower::ServiceExt;

    pub(crate) struct Client {
        pub app: Router,
        pub cookie: Option<String>,
    }

    /// Fresh app on a temp dir. Returns (router, temp dir, storage dir); remove the temp dir when done.
    pub(crate) async fn test_app(max_upload: usize, default_quota: Option<i64>) -> (Router, PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("minidrive-test-{}", uuid::Uuid::new_v4()));
        let storage_dir = dir.join("files");
        tokio::fs::create_dir_all(&storage_dir).await.unwrap();
        let db = connect_db(&format!("sqlite://{}", dir.join("app.db").display())).await.unwrap();
        let app = router(Arc::new(App {
            db,
            storage_dir: storage_dir.clone(),
            jwt_secret: "test".into(),
            max_upload,
            cookie_secure: false,
            default_quota,
        }));
        (app, dir, storage_dir)
    }

    /// Registers and logs in a user.
    pub(crate) async fn user(app: &Router, name: &str) -> Client {
        let mut c = Client { app: app.clone(), cookie: None };
        let creds = json!({ "username": name, "password": "password123" });
        c.json("POST", "/api/auth/register", creds.clone()).await;
        c.json("POST", "/api/auth/login", creds).await;
        c
    }

    impl Client {
        pub(crate) async fn send(&mut self, method: &str, uri: &str, ctype: &str, body: Body) -> (StatusCode, Vec<u8>) {
            let mut req = Request::builder().method(method).uri(uri).header("content-type", ctype);
            if let Some(c) = &self.cookie {
                req = req.header("cookie", c);
            }
            let res = self.app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
            if let Some(sc) = res.headers().get("set-cookie") {
                self.cookie = Some(sc.to_str().unwrap().split(';').next().unwrap().to_string());
            }
            let status = res.status();
            (status, res.into_body().collect().await.unwrap().to_bytes().to_vec())
        }

        pub(crate) async fn json(&mut self, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
            let (s, b) = self.send(method, uri, "application/json", Body::from(body.to_string())).await;
            (s, serde_json::from_slice(&b).unwrap_or(Value::Null))
        }

        pub(crate) async fn upload(&mut self, folder: i64, name: &str, data: &[u8]) -> (StatusCode, Value) {
            let mut body = format!(
                "--X\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: text/plain\r\n\r\n"
            )
            .into_bytes();
            body.extend_from_slice(data);
            body.extend_from_slice(b"\r\n--X--\r\n");
            let uri = format!("/api/files/upload?folder_id={folder}");
            let (s, b) = self.send("POST", &uri, "multipart/form-data; boundary=X", Body::from(body)).await;
            (s, serde_json::from_slice(&b).unwrap_or(Value::Null))
        }
    }

    #[tokio::test]
    async fn full_flow() {
        let (app, dir, storage_dir) = test_app(1024, None).await;
        let mut alice = Client { app: app.clone(), cookie: None };
        let mut bob = Client { app, cookie: None };
        let creds = json!({ "username": "alice", "password": "password123" });

        // auth
        assert_eq!(alice.json("POST", "/api/auth/register", creds.clone()).await.0, StatusCode::CREATED);
        assert_eq!(alice.json("POST", "/api/auth/register", creds.clone()).await.0, StatusCode::CONFLICT);
        assert_eq!(alice.json("GET", "/api/auth/me", json!({})).await.0, StatusCode::UNAUTHORIZED);
        let bad = json!({ "username": "alice", "password": "wrongpass1" });
        assert_eq!(alice.json("POST", "/api/auth/login", bad).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(alice.json("POST", "/api/auth/login", creds).await.0, StatusCode::OK);
        assert_eq!(alice.json("GET", "/api/auth/me", json!({})).await.1["username"], "alice");

        // folders + upload
        let (s, docs) = alice.json("POST", "/api/folders", json!({ "name": "Docs" })).await;
        assert_eq!(s, StatusCode::CREATED);
        let docs = docs["id"].as_i64().unwrap();
        let (_, work) = alice.json("POST", "/api/folders", json!({ "name": "Work", "parent_id": docs })).await;
        let work = work["id"].as_i64().unwrap();
        let (s, file) = alice.upload(work, "hello world.txt", b"hello").await;
        assert_eq!(s, StatusCode::CREATED);
        let file_id = file["id"].as_i64().unwrap();
        assert_eq!(file["size_bytes"], 5);
        assert_eq!(std::fs::read_dir(&storage_dir).unwrap().count(), 1);

        // list + breadcrumbs
        let (_, listing) = alice.json("GET", &format!("/api/files?folder_id={work}"), json!({})).await;
        assert_eq!(listing["files"][0]["name"], "hello world.txt");
        assert_eq!(listing["breadcrumbs"], json!([{ "id": docs, "name": "Docs" }, { "id": work, "name": "Work" }]));
        let (_, root) = alice.json("GET", "/api/files", json!({})).await;
        assert_eq!(root["folders"][0]["name"], "Docs");

        // download
        let (s, bytes) = alice.send("GET", &format!("/api/files/download/{file_id}"), "", Body::empty()).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(bytes, b"hello");

        // another user sees nothing
        let creds = json!({ "username": "bob", "password": "password123" });
        bob.json("POST", "/api/auth/register", creds.clone()).await;
        bob.json("POST", "/api/auth/login", creds).await;
        let (s, _) = bob.send("GET", &format!("/api/files/download/{file_id}"), "", Body::empty()).await;
        assert_eq!(s, StatusCode::NOT_FOUND);
        assert_eq!(bob.json("DELETE", &format!("/api/folders/{docs}"), json!({})).await.0, StatusCode::NOT_FOUND);
        assert_eq!(bob.upload(work, "x.txt", b"x").await.0, StatusCode::NOT_FOUND);
        assert_eq!(bob.json("GET", "/api/files", json!({})).await.1["folders"], json!([]));

        // rename + validation
        let uri = format!("/api/files/{file_id}");
        assert_eq!(alice.json("PATCH", &uri, json!({ "name": "renamed.txt" })).await.0, StatusCode::NO_CONTENT);
        assert_eq!(alice.json("PATCH", &uri, json!({ "name": "a/b" })).await.0, StatusCode::BAD_REQUEST);

        // too large
        let (s, _) = alice.upload(work, "big.bin", &[0u8; 2048]).await;
        assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(std::fs::read_dir(&storage_dir).unwrap().count(), 1, "partial upload left on disk");

        // folder delete trashes the subtree; purging it removes rows and disk files
        assert_eq!(alice.json("DELETE", &format!("/api/folders/{docs}"), json!({})).await.0, StatusCode::NO_CONTENT);
        assert_eq!(alice.json("PATCH", &uri, json!({ "name": "x" })).await.0, StatusCode::NOT_FOUND);
        assert_eq!(std::fs::read_dir(&storage_dir).unwrap().count(), 1);
        assert_eq!(alice.json("DELETE", &format!("/api/trash/folders/{docs}"), json!({})).await.0, StatusCode::NO_CONTENT);
        assert_eq!(std::fs::read_dir(&storage_dir).unwrap().count(), 0);

        // logout clears cookie
        alice.json("POST", "/api/auth/logout", json!({})).await;
        assert_eq!(alice.json("GET", "/api/auth/me", json!({})).await.0, StatusCode::UNAUTHORIZED);

        let _ = std::fs::remove_dir_all(dir);
    }
}
