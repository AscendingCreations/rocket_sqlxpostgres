use log::LevelFilter;
use rocket::{
    fairing::{self, Fairing, Info},
    http::Status,
    outcome::{try_outcome, Outcome},
    request::{FromRequest, Request},
    Build, Rocket, State,
};
use sqlx::{
    postgres::{PgConnectOptions, PgPool, PgPoolOptions},
    ConnectOptions,
};

use std::borrow::Cow;

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

impl SqlxPostgresConfig {
    /// Set database pools max connections limit.
    ///
    /// Call on the fairing before passing it to `rocket.attach()`
    pub fn set_max_connections(mut self, max: u32) -> Self {
        let max = std::cmp::max(max, 1);
        self.max_connections = max;
        self
    }

    /// Set database name
    ///
    /// Call on the fairing before passing it to `rocket.attach()`
    pub fn with_database(mut self, database: impl Into<Cow<'static, str>>) -> Self {
        self.database = database.into();
        self
    }

    /// Set username
    ///
    /// Call on the fairing before passing it to `rocket.attach()`
    pub fn with_username(mut self, username: impl Into<Cow<'static, str>>) -> Self {
        self.username = username.into();
        self
    }

    /// Set user password
    ///
    /// Call on the fairing before passing it to `rocket.attach()`
    pub fn with_password(mut self, password: impl Into<Cow<'static, str>>) -> Self {
        self.password = password.into();
        self
    }

    /// Set database hostname
    ///
    /// Call on the fairing before passing it to `rocket.attach()`
    pub fn with_host(mut self, host: impl Into<Cow<'static, str>>) -> Self {
        self.host = host.into();
        self
    }

    /// Set database port
    ///
    /// Call on the fairing before passing it to `rocket.attach()`
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set database logging level
    ///
    /// Call on the fairing before passing it to `rocket.attach()`
    pub fn with_loglevel(mut self, level: LevelFilter) -> Self {
        self.log_level = level;
        self
    }
}

#[derive(Debug)]
pub struct SQLxPostgresPool {
    pub client: PgPool,
}

impl SQLxPostgresPool {
    pub fn new(client: PgPool) -> Self {
        Self { client }
    }
}

#[derive(Debug)]
pub struct SQLxPostgres {
    pub poll: PgPool,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SQLxPostgres {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, (Status, Self::Error), ()> {
        let store = try_outcome!(request.guard::<&State<SQLxPostgresPool>>().await);
        Outcome::Success(SQLxPostgres {
            poll: store.client.clone(),
        })
    }
}

/// Fairing struct
#[derive(Default)]
pub struct SqlxPostgresFairing {
    poll: Option<PgPool>,
    config: SqlxPostgresConfig,
}

impl SqlxPostgresFairing {
    /// Creates a New SQLx Postgres Fairing.
    ///
    /// Must be Created to pass to `rocket.attach()`
    pub fn new(config: SqlxPostgresConfig, poll: Option<PgPool>) -> Self {
        Self { poll, config }
    }
}

#[rocket::async_trait]
impl Fairing for SqlxPostgresFairing {
    fn info(&self) -> Info {
        Info {
            name: "SQLxPostgres",
            kind: fairing::Kind::Ignite,
        }
    }

    async fn on_ignite(
        &self,
        rocket: Rocket<Build>,
    ) -> std::result::Result<Rocket<Build>, Rocket<Build>> {
        let store = if let Some(poll) = &self.poll {
            SQLxPostgresPool::new(poll.clone())
        } else {
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

            SQLxPostgresPool::new(pg_pool)
        };

        Ok(rocket.manage(store))
    }
}
