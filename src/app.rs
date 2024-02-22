use crate::util::unix_timestamp;
use crate::{entities, retry_lock::RetryLock, task_pool::TaskPool};
use anyhow::anyhow;
use entities::applications;
use entities::applications::Entity as ApplicationEntity;
use sea_orm::{ActiveValue, ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use self::process::ApplicationProcess;

pub mod process;

#[derive(Clone)]
pub struct ApplicationState<Extension>
where
    Extension: Clone,
{
    pub database: DatabaseConnection,
    pub tasks: TaskPool,
    pub locks: RetryLock,
    pub extension: Extension,
}

#[derive(Clone)]
pub struct Application<Extension>
where
    Extension: Clone,
{
    pub state: ApplicationState<Extension>,
    record: applications::Model,
}

impl<Extension> Application<Extension>
where
    Extension: Clone,
{
    /// register an application into the database
    pub async fn register(
        name: &str,
        host: &str,
        app_state: &ApplicationState<Extension>,
    ) -> anyhow::Result<Application<Extension>> {
        let timestamp = unix_timestamp();
        let seed = format!("{}||{}||{}||{}", timestamp, name, host, Uuid::new_v4());
        let seed_secret = format!("{}{}{}{}{}", seed, timestamp, name, timestamp, Uuid::new_v4());

        let hash = format!("{:x}", md5::compute(seed));
        let hash_secret = format!("{:x}", md5::compute(seed_secret));

        let new_app = applications::ActiveModel {
            id: ActiveValue::NotSet,
            hash: ActiveValue::Set(hash),
            hash_secret: ActiveValue::Set(hash_secret),
            name: ActiveValue::Set(name.to_string()),
            host: ActiveValue::Set(host.to_string()),
            created_at: ActiveValue::Set(timestamp),
            updated_at: ActiveValue::Set(0),
            deleted_at: ActiveValue::Set(0),
        };

        let app = ApplicationEntity::insert(new_app).exec(&app_state.database).await?;
        let application = ApplicationEntity::find_by_id(app.last_insert_id)
            .one(&app_state.database)
            .await?;

        if let Some(record) = application {
            Ok(Application {
                state: app_state.clone(),
                record,
            })
        } else {
            Err(anyhow!("Failed to register application"))
        }
    }

    /// gets application record information based off the supplied identifiying hash and secret
    pub async fn get(
        hash: &str,
        secret: &str,
        app_state: &ApplicationState<Extension>,
    ) -> anyhow::Result<Application<Extension>> {
        let model = ApplicationEntity::find()
            .filter(
                Condition::all()
                    .add(applications::Column::Hash.eq(hash))
                    .add(applications::Column::HashSecret.eq(secret))
                    .add(applications::Column::DeletedAt.eq(0)),
            )
            .one(&app_state.database)
            .await?;

        if let Some(record) = model {
            Ok(Application {
                state: app_state.clone(),
                record,
            })
        } else {
            Err(anyhow!("Unable to authorize application credentials"))
        }
    }

    /// get the name of the application
    pub fn name(&self) -> &str {
        self.record.name.as_str()
    }

    /// get the host tied to the application
    pub fn host(&self) -> &str {
        self.record.host.as_str()
    }

    /// get the desired process
    pub async fn process(&self, name: &str) -> anyhow::Result<ApplicationProcess<Extension>> {
        ApplicationProcess::get(self, name).await
    }
}

#[cfg(test)]
mod tests {

    use super::ApplicationState;
    use crate::app::process::LogLevel;
    use crate::app::Application;
    use crate::database;
    use crate::retry_lock::RetryLock;
    use crate::task_pool::TaskPool;
    use crate::tokio;

    #[derive(Clone, Default)]
    struct DemoExtension {
        pub a: i32,
        pub b: i32,
        pub c: i32,
    }

    #[tokio::test]
    pub async fn appstate_test() {
        tracing::info!("Setting up database connection");
        let db = database::connect("mysql://root@localhost/levelcrush", 1).await;

        let state = ApplicationState::<DemoExtension> {
            database: db,
            tasks: TaskPool::new(1),
            locks: RetryLock::default(),
            extension: DemoExtension::default(),
        };

        let _ = state.database.close().await;
    }

    #[tokio::test]
    pub async fn appstate_noextension_test() {
        tracing::info!("Setting up database connection");
        let db = database::connect("mysql://root@localhost/levelcrush", 1).await;

        let state = ApplicationState::<()> {
            database: db,
            tasks: TaskPool::new(1),
            locks: RetryLock::default(),
            extension: (),
        };

        let _ = state.database.close().await;
    }

    #[tokio::test]
    pub async fn app_register_test() {
        tracing::info!("Setting up database connection");
        let db = database::connect("mysql://root@localhost/levelcrush", 1).await;
        let state = ApplicationState::<()> {
            database: db,
            tasks: TaskPool::new(1),
            locks: RetryLock::default(),
            extension: (),
        };

        let app = Application::register("mock", "localhost", &state)
            .await
            .expect("Application did not create");

        let global_process = app.process("global").await.expect("No process found or created");

        // in this case we are going to opt to wait on the handle that returns
        // but we do not need to actually do this in a real application
        let handle = global_process.log(LogLevel::Info, "Hello World!", None);
        let _ = handle.await;
    }

    #[tokio::test]
    pub async fn app_log_test() {
        tracing::info!("Setting up database connection");
        let db = database::connect("mysql://root@localhost/levelcrush", 1).await;
        let state = ApplicationState::<()> {
            database: db,
            tasks: TaskPool::new(1),
            locks: RetryLock::default(),
            extension: (),
        };

        let app = Application::register("mock", "localhost", &state)
            .await
            .expect("Application did not create");

        let global_process = app.process("global").await.expect("No process found or created");

        // in this case we are going to opt to wait on the handle that returns
        // but we do not need to actually do this in a real application
        let mut logs = Vec::new();
        logs.push(global_process.log(LogLevel::Info, "Hello World!", None));
        logs.push(global_process.log(LogLevel::Warning, "Warn World!", None));
        logs.push(global_process.log(LogLevel::Error, "Error World!", None));
        logs.push(global_process.log(LogLevel::Debug, "Debug World!", None));

        let _ = futures::future::join_all(logs).await;
    }
}
