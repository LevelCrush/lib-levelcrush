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

pub mod alias;
pub mod app;
pub mod cache;
pub mod database;
pub mod entities;
pub mod macros;
pub mod retry_lock;
pub mod server;
pub mod task_pool;
pub mod util;

pub mod env;

pub use entities::prelude::*;
//pub use sea_orm::EntityTrait;

/// setups tracing and loads settings from the local .env file
pub fn env() {
    // merge env file into std::env
    dotenvy::dotenv().ok();

    // setup better tracing
    tracing_subscriber::fmt::init();
}

#[cfg(test)]
mod test {

    use crate::database;
    use crate::tokio;

    #[tokio::test]
    pub async fn test_app() {
        let db = database::connect("mysql://root@localhost/levelcrush", 1).await;
        tracing::info!("Testing connection");
        let r = db.ping().await;

        if r.is_err() {
            tracing::info!("Bad connection");
        } else {
            tracing::info!("Good connection");
        }
    }
}
