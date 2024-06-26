use std::collections::HashMap;

use crate::env::{self, EnvVar};
use crate::util::unix_timestamp;
use crate::{entities, retry_lock::RetryLock, task_pool::TaskPool};
use anyhow::{anyhow, Result};
use entities::applications;
use entities::applications::Entity as ApplicationEntity;
use migration::IndexCreateStatement;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use self::process::ApplicationProcess;

pub mod process;
pub mod settings;

#[derive(Clone)]
pub struct ApplicationState<Extension>
where
    Extension: Clone,
{
    pub database: DatabaseConnection,
    pub database_core: DatabaseConnection,
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

        let app = ApplicationEntity::insert(new_app)
            .exec(&app_state.database_core)
            .await?;
        let application = ApplicationEntity::find_by_id(app.last_insert_id)
            .one(&app_state.database_core)
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

    /// regenerate the secret tied to this application.
    /// useful in case credentials get leaked
    pub async fn regen_hashes(&mut self) -> anyhow::Result<()> {
        let timestamp = unix_timestamp();
        let name = self.record.name.clone();
        let host = self.record.host.clone();
        let seed = format!("{}||{}||{}||{}", timestamp, name, host, Uuid::new_v4());
        let seed_secret = format!("{}{}{}{}{}", seed, timestamp, name, timestamp, Uuid::new_v4());
        let hash = format!("{:x}", md5::compute(seed));
        let hash_secret = format!("{:x}", md5::compute(seed_secret));

        self.record.hash = hash;
        self.record.hash_secret = hash_secret;
        self.record.updated_at = unix_timestamp();

        let active: applications::ActiveModel = self.record.clone().into();
        active.save(&self.state.database_core).await?;

        Ok(())
    }

    /// gets application record information based off the supplied identifiying hash and secret
    pub async fn get(
        hash: &str,
        secret: &str,
        app_state: &ApplicationState<Extension>,
    ) -> anyhow::Result<Option<Application<Extension>>> {
        let model = ApplicationEntity::find()
            .filter(
                Condition::all()
                    .add(applications::Column::Hash.eq(hash))
                    .add(applications::Column::HashSecret.eq(secret))
                    .add(applications::Column::DeletedAt.eq(0)),
            )
            .one(&app_state.database_core)
            .await?;

        if let Some(record) = model {
            Ok(Some(Application {
                state: app_state.clone(),
                record,
            }))
        } else {
            Ok(None)
        }
    }

    /// Attempts to autoload  the application based off the application .env settings
    /// If no application can be found. It will register a new one and save the information into the .env
    pub async fn env(state: &ApplicationState<Extension>) -> anyhow::Result<Application<Extension>> {
        let application_id = env::get(EnvVar::ApplicationID);
        let application_secret = env::get(EnvVar::ApplicationSecret);
        let application_name = env::get(EnvVar::ApplicationName);
        let application_host = env::get(EnvVar::ApplicationHost);

        let database_string_core = env::get(EnvVar::DatabaseUrlCore);
        let database_string_self = env::get(EnvVar::DatabaseUrlSelf);

        let my_app = if let Some(existing_app) = Application::get(&application_id, &application_secret, state).await? {
            existing_app
        } else {
            let new_app = Application::register(&application_name, &application_host, state).await?;

            let new_settings = vec![
                (EnvVar::ApplicationID.into(), new_app.record.hash.clone()),
                (EnvVar::ApplicationSecret.into(), new_app.record.hash_secret.clone()),
                (EnvVar::ApplicationName.into(), new_app.record.name.clone()),
                (EnvVar::ApplicationHost.into(), new_app.record.host.clone()),
                (EnvVar::DatabaseUrlCore.into(), database_string_core),
                (EnvVar::DatabaseUrlSelf.into(), database_string_self),
            ];
            let new_settings: HashMap<&'static str, String> = new_settings.into_iter().collect();

            crate::env::save(new_settings).await?;

            new_app
        };

        Ok(my_app)
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

    use tracing_test::traced_test;

    use super::ApplicationState;
    use crate::app::process::LogLevel;
    use crate::app::settings::{ApplicationSettingType, ApplicationSettings};
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

    #[traced_test]
    #[tokio::test]
    pub async fn appstate_test() {
        tracing::info!("Setting up database connection");
        let db = database::connect("mysql://root@localhost/levelcrush", 1).await;

        let state = ApplicationState::<DemoExtension> {
            database: db.clone(),
            database_core: db,
            tasks: TaskPool::new(1),
            locks: RetryLock::default(),
            extension: DemoExtension::default(),
        };

        let _ = state.database.close().await;
    }

    #[traced_test]
    #[tokio::test]
    pub async fn appstate_noextension_test() {
        tracing::info!("Setting up database connection");
        let db = database::connect("mysql://root@localhost/levelcrush", 1).await;

        let state = ApplicationState::<()> {
            database: db.clone(),
            database_core: db,
            tasks: TaskPool::new(1),
            locks: RetryLock::default(),
            extension: (),
        };

        let _ = state.database.close().await;
    }
    #[traced_test]
    #[tokio::test]
    pub async fn app_register_test() {
        tracing::info!("Setting up database connection");
        let db = database::connect("mysql://root@localhost/levelcrush", 1).await;
        let state = ApplicationState::<()> {
            database: db.clone(),
            database_core: db,
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

    #[traced_test]
    #[tokio::test]
    pub async fn app_log_test() {
        tracing::info!("Setting up database connection");
        let db = database::connect("mysql://root@localhost/levelcrush", 1).await;
        let state = ApplicationState::<()> {
            database: db.clone(),
            database_core: db,
            tasks: TaskPool::new(1),
            locks: RetryLock::default(),
            extension: (),
        };

        let app = Application::register("mock_settings", "localhost", &state)
            .await
            .expect("Application did not create");

        let global_process = app.process("global").await.expect("No process found or created");

        // in this case we are going to opt to wait on the handle that returns
        // but we do not need to actually do this in a real application
        let mut handles = Vec::new();
        handles.push(global_process.log(LogLevel::Info, "Hello World!", None));
        handles.push(global_process.log(LogLevel::Warning, "Warn World!", None));
        handles.push(global_process.log(LogLevel::Error, "Error World!", None));
        handles.push(global_process.log(LogLevel::Debug, "Debug World!", None));

        let _ = futures::future::join_all(handles).await;
    }

    #[traced_test]
    #[tokio::test]
    pub async fn app_env_load_test() {
        let _ = dotenvy::dotenv();

        tracing::info!("Setting up database connection");
        let db = database::connect("mysql://root@localhost/levelcrush", 1).await;
        let state = ApplicationState::<()> {
            database: db.clone(),
            database_core: db,
            tasks: TaskPool::new(1),
            locks: RetryLock::default(),
            extension: (),
        };

        let app = Application::env(&state).await.expect("Application did not create");

        let global_process = app.process("global").await.expect("No process found or created");

        // in this case we are going to opt to wait on the handle that returns
        // but we do not need to actually do this in a real application
        let mut handles = Vec::new();
        handles.push(global_process.log(LogLevel::Info, "Hello World!", None));
        handles.push(global_process.log(LogLevel::Warning, "Warn World!", None));
        handles.push(global_process.log(LogLevel::Error, "Error World!", None));
        handles.push(global_process.log(LogLevel::Debug, "Debug World!", None));

        let _ = futures::future::join_all(handles).await;
    }

    /// this is a bad test. It is really just here to test functionality.
    /// todo: rewrite this for proper test
    #[traced_test]
    #[tokio::test]
    pub async fn appsetting_test() -> anyhow::Result<()> {
        tracing::info!("Beginning setting test");

        let db = database::connect("mysql://root@localhost/levelcrush", 1).await;
        let state = ApplicationState::<()> {
            database: db.clone(),
            database_core: db,
            tasks: TaskPool::new(1),
            locks: RetryLock::default(),
            extension: (),
        };

        let app = if let Some(app) = Application::get(
            "082fd059b5e0e43df8710065cd9e6cea", // replace with your own hash
            "2576234c0ba66a83737d88848b0a9011", // replace with your own hash secret
            &state,
        )
        .await?
        {
            tracing::info!("found existing application");
            app
        } else {
            tracing::info!("Registering new application");
            Application::register("mock_test", "localhost", &state)
                .await
                .expect("Application did not create")
        };

        let global_process = app.process("global").await.expect("No process found or created");

        global_process
            .log(LogLevel::Info, "Starting to load settings", None)
            .await?;

        tracing::info!("Loading application settings");

        let mut settings = ApplicationSettings::load(&app).await?;

        // precheck settings
        let test1 = settings.get_global("mock.test1");
        let test2 = settings.get_global("mock.test2");

        tracing::info!("Test pre");
        tracing::info!("{:?}", test1);
        tracing::info!("{:?}", test2);

        // set default settings
        futures::future::join_all(vec![
            settings
                .set(ApplicationSettingType::Global, "mock.test1", "hello world", None)
                .await?,
            settings
                .set(ApplicationSettingType::Global, "mock.test2", "foobar", None)
                .await?,
            settings
                .set(ApplicationSettingType::Global, "mock.test3", "global_happy", None)
                .await?,
        ])
        .await;

        // load global settings
        let test1 = settings.get_global("mock.test1");
        let test2 = settings.get_global("mock.test2");

        tracing::info!("Test post");
        tracing::info!("{:?}", test1);
        tracing::info!("{:?}", test2);

        tracing::info!("Modifying set");
        futures::future::join_all(vec![
            settings
                .set(ApplicationSettingType::Global, "mock.test1", "modified test 1", None)
                .await?,
            settings
                .set(ApplicationSettingType::Global, "mock.test2", "modified test 2", None)
                .await?,
        ])
        .await;

        // load global settings
        let test1 = settings.get_global("mock.test1");
        let test2 = settings.get_global("mock.test2");

        tracing::info!("Test global mod");
        tracing::info!("{:?}", test1);
        tracing::info!("{:?}", test2);

        tracing::info!("Setting user settings");
        futures::future::join_all(vec![
            settings
                .set(
                    ApplicationSettingType::User,
                    "mock.test1",
                    "user test 1",
                    Some("123".to_string()),
                )
                .await?,
            settings
                .set(
                    ApplicationSettingType::User,
                    "mock.test2",
                    "user test 2",
                    Some("123".to_string()),
                )
                .await?,
        ])
        .await;

        // load global settings
        let test1 = settings.get_user("123", "mock.test1");
        let test2 = settings.get_user("123", "mock.test2");
        let test3 = settings.get_user("123", "mock.test3");
        tracing::info!("Test user  mod");
        tracing::info!("{:?}", test1);
        tracing::info!("{:?}", test2);
        tracing::info!("{:?}", test3);
        Ok(())
    }
}
