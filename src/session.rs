use rocket::{
    fairing::{self, Fairing, Info},
    http::Status,
    outcome::Outcome,
    request::FromRequest,
    try_outcome, Request, Rocket, State,
};
use sqlx::{
    pool::PoolConnection,
    postgres::{PgConnectOptions, PgPool, PgPoolOptions},
    ConnectOptions,
};
use log::LevelFilter;

use std::borrow::Cow;

pub use anyhow::Error;
/// An anyhow::Result with default return type of ()
pub type Result<T = ()> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct SqlxPostgresConfig {
    /// Database name
    database: Cow<'static, str>,
    /// Database username for login
    username: Cow<'static, str>,
    /// Database password for login
    password: Cow<'static, str>,
    /// Database Host address
    host: Cow<'static, str>,
    /// Database Port address
    port: u16,
    /// Database Max Poll Connections.
    max_connections: u32,
    /// Log Level for the database
    log_level: LevelFilter,
}

impl Default for SqlxPostgresConfig {
    fn default() -> Self {
        Self {
            database: "".into(),
            username: "".into(),
            password: "".into(),
            host: "localhost".into(),
            port: 5432,
            max_connections: 5,
            log_level: LevelFilter::Debug,
        }
    }
}

#[derive(Debug)]
pub struct SQLxPostgresPoller {
    pub client: PgPool,
}

impl SQLxPostgresPoller {
    pub fn new(client: PgPool) -> Self {
        Self { client,}
    }

    async fn connection(&self) -> sqlx::Result<PoolConnection<sqlx::Postgres>> {
        self.client.acquire().await
    }
}

#[derive(Debug)]
pub struct SQLxPostgres {
    pub poll: PgPool,
}

#[rocket::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for SQLxPostgres {
    type Error = ();

    async fn from_request(request: &'a Request<'r>) -> Outcome<Self, (Status, Self::Error), ()> {
        let store: State<SQLxPostgresPoller> = try_outcome!(request.guard().await);
        Outcome::Success(SQLxPostgres {
            poll: store.client.clone(),
        })
    }
}

/// Fairing struct
#[derive(Default)]
pub struct SqlxPostgresFairing {
    config: SqlxPostgresConfig,
}

impl SqlxPostgresFairing {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set session database pools max connections limit.
    ///
    /// Call on the fairing before passing it to `rocket.attach()`
    pub fn set_max_connections(mut self, max: u32) -> Self {
        let max = std::cmp::max(max, 1);
        self.config.max_connections = max;
        self
    }

    /// Set session database name
    ///
    /// Call on the fairing before passing it to `rocket.attach()`
    pub fn with_database(mut self, database: impl Into<Cow<'static, str>>) -> Self {
        self.config.database = database.into();
        self
    }

    /// Set session username
    ///
    /// Call on the fairing before passing it to `rocket.attach()`
    pub fn with_username(mut self, username: impl Into<Cow<'static, str>>) -> Self {
        self.config.username = username.into();
        self
    }

    /// Set session user password
    ///
    /// Call on the fairing before passing it to `rocket.attach()`
    pub fn with_password(mut self, password: impl Into<Cow<'static, str>>) -> Self {
        self.config.password = password.into();
        self
    }

    /// Set session database hostname
    ///
    /// Call on the fairing before passing it to `rocket.attach()`
    pub fn with_host(mut self, host: impl Into<Cow<'static, str>>) -> Self {
        self.config.host = host.into();
        self
    }

    /// Set session database port
    ///
    /// Call on the fairing before passing it to `rocket.attach()`
    pub fn with_port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    /// Set session database logging level
    ///
    /// Call on the fairing before passing it to `rocket.attach()`
    pub fn with_loglevel(mut self, level: LevelFilter) -> Self {
        self.config.log_level = level;
        self
    }
}

#[rocket::async_trait]
impl Fairing for SqlxPostgresFairing {
    fn info(&self) -> Info {
        Info {
            name: "SQLxPostgres",
            kind: fairing::Kind::Attach,
        }
    }

    async fn on_attach(&self, rocket: Rocket) -> std::result::Result<Rocket, Rocket> {
        let mut connect_opts = PgConnectOptions::new();
        connect_opts.log_statements(self.config.log_level);
        connect_opts = connect_opts.database(&self.config.database[..]);
        connect_opts = connect_opts.username(&self.config.username[..]);
        connect_opts = connect_opts.password(&self.config.password[..]);
        connect_opts = connect_opts.host(&self.config.host[..]);
        connect_opts = connect_opts.port(self.config.port);

        let pg_pool = match PgPoolOptions::new()
            .max_connections(self.config.max_connections)
            .connect_with(connect_opts)
            .await
        {
            Ok(n) => n,
            Err(_) => return Ok(rocket),
        };

        let store = SQLxPostgresPoller::new(pg_pool);

        Ok(rocket.manage(store))
    }
}
