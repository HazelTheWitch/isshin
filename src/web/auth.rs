use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Form, Json, Router,
};
use password_auth::generate_hash;
use serde::{Deserialize, Serialize};
use sqlx::error::ErrorKind;

use crate::{
    state::AppState,
    user::{AuthSession, Credentials},
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
}

pub fn protected() -> Router<Arc<AppState>> {
    Router::new().route("/register", post(register))
}

async fn login(mut auth_session: AuthSession, Form(creds): Form<Credentials>) -> StatusCode {
    let user = match auth_session.authenticate(creds.clone()).await {
        Ok(Some(user)) => user,
        Ok(None) => return StatusCode::FORBIDDEN,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    if auth_session.login(&user).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

async fn logout(mut auth_session: AuthSession) -> StatusCode {
    match auth_session.logout() {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[derive(Serialize, Deserialize)]
pub enum RegisterResponse {
    Ok,
    UsernameTaken,
    BadPassword { zxcvbn: u8 },
}

async fn register(
    State(state): State<Arc<AppState>>,
    Form(Credentials { username, password }): Form<Credentials>,
) -> Response {
    let entropy = match zxcvbn::zxcvbn(&password, &[&username]) {
        Ok(entropy) => entropy,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if entropy.score() < 3 {
        return Json(RegisterResponse::BadPassword {
            zxcvbn: entropy.score(),
        })
        .into_response();
    }

    let result = sqlx::query("INSERT INTO users (username, password) VALUES (?, ?);")
        .bind(username)
        .bind(generate_hash(password))
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => Json(RegisterResponse::Ok).into_response(),
        Err(sqlx::Error::Database(err)) => match err.kind() {
            ErrorKind::UniqueViolation => Json(RegisterResponse::UsernameTaken).into_response(),
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
