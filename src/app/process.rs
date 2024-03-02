use super::Application;
use crate::entities::application_process_logs;
use crate::{
    entities::{self, application_processes},
    util::unix_timestamp,
};
use anyhow::anyhow;
use application_processes::Entity as ApplicationProcessEntity;
use sea_orm::{ActiveValue, ColumnTrait, Condition, EntityTrait, QueryFilter};
use tokio::task::JoinHandle;

#[repr(i8)]
pub enum LogLevel {
    Error = 0,
    Warning = 1,
    Info = 2,
    Debug = 3,
}

#[derive(Clone)]
pub struct ApplicationProcess<Extension>
where
    Extension: Clone,
{
    application: Application<Extension>,
    record: application_processes::Model,
}

impl<Extension> ApplicationProcess<Extension>
where
    Extension: Clone,
{
    /// get the application process by supplying the application and name
    pub async fn get(
        application: &Application<Extension>,
        name: &str,
    ) -> anyhow::Result<ApplicationProcess<Extension>> {
        let model_process = ApplicationProcessEntity::find()
            .filter(
                Condition::all()
                    .add(application_processes::Column::Application.eq(application.record.id))
                    .add(application_processes::Column::Name.eq(name))
                    .add(application_processes::Column::DeletedAt.eq(0)),
            )
            .one(&application.state.database_core)
            .await?;

        if let Some(record) = model_process {
            return Ok(ApplicationProcess {
                application: application.clone(),
                record,
            });
        }

        // no application process existing tied to application and name.
        // create new application record

        let timestamp = unix_timestamp();
        let seed = format!("{}||{}||{}", timestamp, application.record.hash_secret, name);
        let hash = format!("{:x}", md5::compute(seed));

        let new_process = application_processes::ActiveModel {
            id: ActiveValue::NotSet,
            application: ActiveValue::Set(application.record.id),
            hash: ActiveValue::Set(hash),
            name: ActiveValue::Set(name.to_string()),
            created_at: ActiveValue::Set(timestamp),
            updated_at: ActiveValue::Set(0),
            deleted_at: ActiveValue::Set(0),
        };

        let process = ApplicationProcessEntity::insert(new_process)
            .exec(&application.state.database_core)
            .await?;

        let process_model = ApplicationProcessEntity::find_by_id(process.last_insert_id)
            .one(&application.state.database_core)
            .await?;

        if let Some(record) = process_model {
            Ok(ApplicationProcess {
                application: application.clone(),
                record,
            })
        } else {
            Err(anyhow!("Unable to get process"))
        }
    }

    pub fn log_info(&self, content: &str) -> JoinHandle<()> {
        self.log(LogLevel::Info, content, None)
    }

    pub fn log_warning(&self, content: &str) -> JoinHandle<()> {
        self.log(LogLevel::Warning, content, None)
    }
    pub fn log_error(&self, content: &str) -> JoinHandle<()> {
        self.log(LogLevel::Error, content, None)
    }

    pub fn log_debug(&self, content: &str) -> JoinHandle<()> {
        self.log(LogLevel::Debug, content, None)
    }

    /// log a message to our database
    pub fn log(&self, log_level: LogLevel, content: &str, sub_id: Option<&str>) -> JoinHandle<()> {
        let sub_id = sub_id.unwrap_or("").to_string();
        match log_level {
            LogLevel::Error => {
                tracing::error!("{sub_id}\r\n{content}");
            }
            LogLevel::Warning => {
                tracing::warn!("{sub_id}\r\n{content}");
            }
            LogLevel::Info => {
                tracing::info!("{sub_id}\r\n{content}");
            }
            LogLevel::Debug => {
                tracing::debug!("{sub_id}\r\n{content}");
            }
        }

        let log_type = log_level as i8;
        let application_id = self.application.record.id;
        let process_id = self.record.id;
        let content_cloned = content.to_string();
        let database = self.application.state.database_core.clone();

        // spawn a background task to log this off to a database.
        // we do this so we dont have to wait around for the response
        tokio::spawn(async move {
            let timestamp = unix_timestamp();
            let seed = format!("{}||{}||{}||{}", timestamp, application_id, process_id, content_cloned);
            let hash = format!("{:x}", md5::compute(seed));
            let hash_sub = format!("{:x}", md5::compute(sub_id));
            let new_log = application_process_logs::ActiveModel {
                id: ActiveValue::NotSet,
                application: ActiveValue::Set(application_id),
                process: ActiveValue::Set(process_id),
                hash: ActiveValue::Set(hash),
                hash_sub: ActiveValue::Set(hash_sub),
                r#type: ActiveValue::Set(log_type),
                content: ActiveValue::Set(content_cloned),
                created_at: ActiveValue::Set(timestamp),
                updated_at: ActiveValue::Set(0),
                deleted_at: ActiveValue::Set(0),
            };

            // quite literally don't care about the errors
            let _ = application_process_logs::Entity::insert(new_log).exec(&database).await;
        })
    }
}
