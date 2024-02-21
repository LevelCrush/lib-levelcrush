// rexports
pub use anyhow;
pub use axum;
pub use axum_sessions;
pub use chrono;
pub use clap;
pub use dotenvy;
pub use dotenvy_macro;
pub use futures;
pub use md5;
pub use rand;
pub use reqwest;
pub use serde;
pub use tokio;
pub use tracing;
pub use urlencoding;
pub use uuid;

pub use levelcrush_macros as proc_macros;
pub use {bigdecimal, bigdecimal::BigDecimal, sqlx, sqlx::Sqlite, sqlx::SqlitePool};

pub mod alias;
pub mod cache;
pub mod database;
pub mod entities;
pub mod macros;
pub mod queries;
pub mod retry_lock;
pub mod server;
pub mod task_pool;
pub mod util;

/// setups tracing and loads settings from the local .env file
pub fn env() {
    // merge env file into std::env
    dotenvy::dotenv().ok();

    // setup better tracing
    tracing_subscriber::fmt::init();
}
