use crate::util::unix_timestamp;
use crate::{entities, retry_lock::RetryLock, task_pool::TaskPool};
use anyhow::anyhow;
use entities::applications;
use entities::applications::Entity as ApplicationEntity;
use sea_orm::{ActiveValue, ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

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
}

#[cfg(test)]
mod tests {

    use super::ApplicationState;
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
}
