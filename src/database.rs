use std::time::Duration;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
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

/*
pub fn log_error<T>(query: Result<T, sqlx::Error>) {
    if let Err(query) = query {
        tracing::error!("{}", query);
        //  panic!("Figuring out this error");
    }
}

pub fn need_retry<T>(query: &Result<T, sqlx::Error>) -> bool {
    let mut code = None;
    if let Err(query) = query {
        let db_error = query.as_database_error();
        if let Some(db_error) = db_error {
            code = db_error.code();
        }
    }

    false

    /* the below code was originally for mysql
    TODO: figure out sqlite equivalent
    if let Some(code) = code {
        let code = code.into_owned();
        tracing::error!("SQL Code Detected: {}", code);
        match code.as_str() {
            "104" => true,  // connection reset by peeer
            "1205" => true, // lock wait timeout
            "1213" => true, // deadlock timeout
            _ => false,
        }
    } else {
        false
    }
    */
}
 */
