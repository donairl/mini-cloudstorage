# Mini-Drive

Self-hosted file storage: Rust (Axum) + SQLite backend, Vue 3 frontend. Files live on local disk under UUID names; metadata lives in SQLite.

## Requirements

- Rust (stable, edition 2024) and Cargo
- Node.js 20.19+ or 22.12+ and npm (required by Vite 8)

## Development

Run backend and frontend in two terminals:

```bash
# 1. Backend — http://localhost:3000
cp .env.example .env          # once; edit JWT_SECRET
cd backend
cargo run

# 2. Frontend — http://localhost:5173 (proxies /api to :3000)
cd frontend
npm install
npm run dev
```

Open http://localhost:5173, register an account, and log in.

The database (`backend/data/app.db`) and uploaded files (`backend/data/files/`) are created on first start.

## Production (single process)

The backend serves the built frontend, so only one port is needed:

```bash
cd frontend && npm ci && npm run build
cd ../backend && cargo build --release
JWT_SECRET="$(openssl rand -hex 32)" COOKIE_SECURE=1 ./target/release/minidrive
```

Set `COOKIE_SECURE=1` only when the app is served over HTTPS (e.g. behind a reverse proxy), otherwise the browser will drop the login cookie.

## Docker

```bash
docker build -t minidrive .
docker run -d -p 3000:3000 \
  -e JWT_SECRET="$(openssl rand -hex 32)" \
  -v minidrive-data:/data \
  minidrive
```

The `/data` volume holds both the SQLite database and the uploaded files. Back up that volume to back up everything.

## Configuration

Settings are read from environment variables. To keep them in a file, copy `.env.example` to `.env` (project root or `backend/`); the backend loads it automatically on start, and real environment variables take precedence. For Docker use `docker run --env-file .env`. `.env` is git- and docker-ignored; never commit it.

| Variable        | Default                  | Description                                          |
|-----------------|--------------------------|------------------------------------------------------|
| `JWT_SECRET`    | — (required)             | Secret for signing login tokens. Keep it stable, or users are logged out on restart. |
| `DATABASE_URL`  | `sqlite://data/app.db`   | SQLite database path (created if missing)            |
| `STORAGE_DIR`   | `data/files`             | Directory for uploaded files                         |
| `STATIC_DIR`    | `../frontend/dist`       | Built frontend to serve                              |
| `MAX_UPLOAD_MB` | `100`                    | Maximum upload size                                  |
| `QUOTA_MB`      | `0`                      | Default storage limit per user; `0` = unlimited. Override one user with `UPDATE users SET quota_bytes = … WHERE username = …` (`NULL` = use the default) |
| `COOKIE_SECURE` | `0`                      | `1` adds the `Secure` flag to the auth cookie (HTTPS) |
| `PORT`          | `3000`                   | HTTP port                                            |

## Tests

```bash
cd backend && cargo test
```

## API

All endpoints except register/login and the public `/api/s/…` share endpoints require the `token` cookie set by login. Errors return `{"error": "..."}`.

| Method | Endpoint                      | Description                              |
|--------|-------------------------------|------------------------------------------|
| POST   | `/api/auth/register`          | Create account `{username, password}`    |
| POST   | `/api/auth/login`             | Log in, sets HttpOnly cookie             |
| POST   | `/api/auth/logout`            | Clear cookie                             |
| GET    | `/api/auth/me`                | Current user                             |
| GET    | `/api/files?folder_id=`       | List folder contents + breadcrumbs       |
| GET    | `/api/files?q=`               | Search file and folder names in all folders |
| POST   | `/api/files/upload?folder_id=`| Upload (multipart field `file`)          |
| GET    | `/api/files/download/{id}`    | Download a file                          |
| PATCH  | `/api/files/{id}`             | Rename file `{name}`                     |
| DELETE | `/api/files/{id}`             | Move file to the trash                   |
| POST   | `/api/folders`                | Create folder `{name, parent_id?}`       |
| PATCH  | `/api/folders/{id}`           | Rename folder `{name}`                   |
| DELETE | `/api/folders/{id}`           | Move folder and everything inside to the trash |
| GET    | `/api/trash`                  | List trashed files and folders           |
| DELETE | `/api/trash`                  | Empty the trash (permanent)              |
| POST   | `/api/trash/{files\|folders}/{id}` | Restore from the trash              |
| DELETE | `/api/trash/{files\|folders}/{id}` | Delete permanently                  |
| GET    | `/api/folders/{id}/zip`       | Download folder and everything inside as a zip |
| GET    | `/api/storage`                | `{used_bytes, quota_bytes}` (`null` = unlimited) |
| POST   | `/api/{files\|folders}/{id}/shares` | Create share link `{expires_in_days?, password?}` |
| GET    | `/api/{files\|folders}/{id}/shares` | List a file's or folder's share links |
| DELETE | `/api/shares/{id}`            | Revoke a share link                      |
| GET    | `/api/s/{token}`              | Public: shared file or folder info       |
| GET/POST | `/api/s/{token}/zip`        | Public: shared folder as a zip (POST with form field `password`) |
| POST   | `/api/s/{token}/list`         | Public: list a shared folder `{password?, folder_id?}` |
| GET    | `/api/s/{token}/download`     | Public: download (no password)           |
| POST   | `/api/s/{token}/download`     | Public: download, form field `password`  |

Uploads over the quota fail with `507`. Trashed files keep counting toward the quota until deleted permanently, and their share links stop working while they are in the trash. Share links open at `/s/{token}` in the frontend. For a folder share, add `?file_id=` to the download endpoints to pick a file inside the folder.
