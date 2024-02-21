use crate::{retry_lock::RetryLock, task_pool::TaskPool};
use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct ApplicationState<T: Clone> {
    pub database: DatabaseConnection,
    pub tasks: TaskPool,
    pub locks: RetryLock,
    pub extensions: T,
}

#[cfg(test)]
mod tests {

    use super::ApplicationState;
    use crate::database;
    use crate::retry_lock::RetryLock;
    use crate::task_pool::TaskPool;
    use crate::tokio;

    #[tokio::test]
    pub async fn appstate_test() {
        tracing::info!("Setting up database connection");
        let db = database::connect("mysql://root@localhost/levelcrush", 1).await;

        let state = ApplicationState::<()> {
            database: db,
            tasks: TaskPool::new(1),
            locks: RetryLock::default(),
            extensions: (),
        };

        let _ = state.database.close().await;
    }
}
