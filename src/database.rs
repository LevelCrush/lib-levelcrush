use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    ConnectOptions, SqlitePool,
};
use std::{str::FromStr, time::Duration};
use tracing::log::LevelFilter;

/// connects to the application database based off .env specific variables
pub async fn connect<T: Into<String>>(database_url: T, max_connections: u32) -> SqlitePool {
    let database_url = database_url.into();
    let mut database_options = SqliteConnectOptions::from_str(database_url.as_str()).unwrap();
    database_options = database_options.log_statements(LevelFilter::Off);
    database_options = database_options.log_slow_statements(LevelFilter::Warn, Duration::from_secs(5));

    tracing::info!(
        "Allowing a maximum of {} total connections to the database",
        max_connections
    );

    SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect_with(database_options)
        .await
        .expect("Could not make database connection")

    // connect to database
    /*
    SqlitePool::connect_with(database_options)
        .await
        .expect("Could not make database connection") */
}

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
