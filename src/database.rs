use std::time::Duration;

use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr, RuntimeErr};
use tracing::log::LevelFilter;

/// Connects to the database for this specific application.
pub async fn connect<T: Into<String>>(database_url: T, max_connections: u32) -> DatabaseConnection {
    tracing::info!(
        "Allowing a maximum of {} total connections to the database",
        max_connections
    );

    let database_url = database_url.into();
    let mut database_options = ConnectOptions::new(database_url.as_str());
    database_options
        .max_connections(max_connections)
        .sqlx_logging(false)
        .sqlx_logging_level(LevelFilter::Off)
        .sqlx_slow_statements_logging_settings(LevelFilter::Warn, Duration::from_secs(5));

    Database::connect(database_options)
        .await
        .expect("Failed to connect to database")
}


pub fn log_error<T>(query: Result<T, DbErr>) {
    if let Err(query) = query {
        tracing::error!("{}", query);
        //  panic!("Figuring out this error");
    }
}