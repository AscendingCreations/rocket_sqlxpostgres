# rocket_sqlxpostgres
SQLx postgres pooler for Rocket

```rust
#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use] extern crate rocket;

use rocket_sqlxpostgres::{SqlxPostgresFairing, SqlxPostgresConfig, SQLxPostgres};
use sqlx::pool::PoolConnection;

fn main() {
    let config = SqlxPostgresConfig::default()
            .with_database("databasename")
            .with_username("username")
            .with_password("password")
            .with_host("localhost")
            .with_port("5432");

    rocket::build()
        .attach(SqlxPostgresFairing::new(config, None))
        .mount("/", routes![index])
        .launch();
}

#[get("/")]
async fn index(sqlxsession: SQLxPostgres) -> Option<String> {
    let mut guard: PoolConnection<sqlx::Postgres> = db.poll.acquire().await.unwrap();

    let (sessions,) =
        sqlx::query_as("SELECT COUNT(*) FROM async_sessions")
            .fetch_one(&mut guard)
            .await.unwrap_or((0i64,));

    format!("{} Sessions in Database", sessions)
}
```