use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::{StatusCode, request::Parts},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode, get_current_timestamp};
use serde::{Deserialize, Serialize};
use time::Duration;

use crate::{App, AppError, AppState, Result};

const TTL_DAYS: i64 = 7;

#[derive(Deserialize)]
pub struct Creds {
    username: String,
    password: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct User {
    id: i64,
    username: String,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: i64,
    exp: u64,
}

pub struct AuthUser(pub i64);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, app: &AppState) -> Result<Self> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar.get("token").ok_or_else(AppError::unauthorized)?;
        let key = DecodingKey::from_secret(app.jwt_secret.as_bytes());
        let data = decode::<Claims>(token.value(), &key, &Validation::default())
            .map_err(|_| AppError::unauthorized())?;
        Ok(AuthUser(data.claims.sub))
    }
}

fn cookie(app: &App, value: String, max_age: Duration) -> Cookie<'static> {
    Cookie::build(("token", value))
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/")
        .secure(app.cookie_secure)
        .max_age(max_age)
        .build()
}

pub async fn register(State(app): State<AppState>, Json(c): Json<Creds>) -> Result<(StatusCode, Json<User>)> {
    let username = c.username.trim().to_string();
    if !(3..=32).contains(&username.chars().count()) {
        return Err(AppError::bad("username must be 3-32 characters"));
    }
    if c.password.chars().count() < 8 {
        return Err(AppError::bad("password must be at least 8 characters"));
    }
    let hash = tokio::task::spawn_blocking(move || {
        Argon2::default().hash_password(c.password.as_bytes()).map(|h| h.to_string())
    })
    .await
    .map_err(AppError::internal)?
    .map_err(AppError::internal)?;

    let res = sqlx::query_as("INSERT INTO users (username, password_hash) VALUES (?, ?) RETURNING id, username")
        .bind(username)
        .bind(hash)
        .fetch_one(&app.db)
        .await;
    match res {
        Ok(user) => Ok((StatusCode::CREATED, Json(user))),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            Err(AppError(StatusCode::CONFLICT, "username already taken".into()))
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn login(
    State(app): State<AppState>,
    jar: CookieJar,
    Json(c): Json<Creds>,
) -> Result<(CookieJar, Json<User>)> {
    let invalid = || AppError(StatusCode::UNAUTHORIZED, "invalid username or password".into());
    let row: Option<(i64, String, String)> =
        sqlx::query_as("SELECT id, username, password_hash FROM users WHERE username = ?")
            .bind(c.username.trim())
            .fetch_optional(&app.db)
            .await?;
    let (id, username, hash) = row.ok_or_else(invalid)?;
    let ok = tokio::task::spawn_blocking(move || {
        PasswordHash::new(&hash).is_ok_and(|h| Argon2::default().verify_password(c.password.as_bytes(), &h).is_ok())
    })
    .await
    .map_err(AppError::internal)?;
    if !ok {
        return Err(invalid());
    }

    let claims = Claims { sub: id, exp: get_current_timestamp() + TTL_DAYS as u64 * 86400 };
    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(app.jwt_secret.as_bytes()))
        .map_err(AppError::internal)?;
    let jar = jar.add(cookie(&app, token, Duration::days(TTL_DAYS)));
    Ok((jar, Json(User { id, username })))
}

pub async fn logout(State(app): State<AppState>, jar: CookieJar) -> CookieJar {
    jar.add(cookie(&app, String::new(), Duration::ZERO))
}

pub async fn me(State(app): State<AppState>, AuthUser(uid): AuthUser) -> Result<Json<User>> {
    sqlx::query_as("SELECT id, username FROM users WHERE id = ?")
        .bind(uid)
        .fetch_optional(&app.db)
        .await?
        .map(Json)
        .ok_or_else(AppError::unauthorized)
}
