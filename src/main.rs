use std::{env, str::FromStr};

use isshin::web;
use password_auth::generate_hash;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let db_address =
        env::var("ISSHIN_SQLITE_ADDRESS").unwrap_or_else(|_| String::from("sqlite::memory:"));

    let db_options = SqliteConnectOptions::from_str(&db_address)?.create_if_missing(true);

    let db = SqlitePool::connect_with(db_options).await?;
    sqlx::migrate!().run(&db).await?;

    if let Ok(password) = env::var("ISSHIN_ADMIN_PASSWORD") {
        sqlx::query("REPLACE INTO users (id, username, password) VALUES (1, 'admin', ?);")
            .bind(generate_hash(password))
            .execute(&db)
            .await?;
    }

    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, web::app(db.clone()).await?).await?;

    Ok(())
}
