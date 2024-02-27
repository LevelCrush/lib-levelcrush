use std::collections::HashMap;

use anyhow::anyhow;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, Condition, EntityTrait, QueryFilter};
use tokio::task::JoinHandle;

use super::Application;
use crate::{
    alias::UnixTimestamp,
    entities::{application_global_settings, application_settings, application_user_settings},
    util::unix_timestamp,
};

#[derive(Clone)]
pub struct ApplicationSettings<Extension>
where
    Extension: Clone,
{
    application: Application<Extension>,
    names: HashMap<i64, String>,
    base: HashMap<String, (application_settings::Model, UnixTimestamp)>,
    global: HashMap<String, (application_global_settings::Model, UnixTimestamp)>,
    user: HashMap<(String, String), (application_user_settings::Model, UnixTimestamp)>,
}

pub enum ApplicationSettingType {
    Global,
    User,
}

impl<Extension> ApplicationSettings<Extension>
where
    Extension: Clone,
{
    /// load the core application settinggs and global settings from the database. This is intended ot be run on the first load of the application
    pub async fn load(app: &Application<Extension>) -> anyhow::Result<ApplicationSettings<Extension>> {
        let mut base = HashMap::new();
        let mut global = HashMap::new();
        let mut names = HashMap::new();
        let user = HashMap::new();
        let timestamp = unix_timestamp();

        let core_settings = application_settings::Entity::find()
            .filter(
                Condition::all()
                    .add(application_settings::Column::Application.eq(app.record.id))
                    .add(application_settings::Column::DeletedAt.eq(0)),
            )
            .all(&app.state.database)
            .await?;

        let mut settings_ids = Vec::new();
        for model in core_settings.iter() {
            settings_ids.push(model.id);
            names.insert(model.id, model.name.clone());
            base.insert(model.name.clone(), (model.clone(), timestamp));
        }

        let global_settings = application_global_settings::Entity::find()
            .filter(
                Condition::all()
                    .add(application_global_settings::Column::Application.eq(app.record.id))
                    .add(application_global_settings::Column::Setting.is_in(settings_ids))
                    .add(application_global_settings::Column::DeletedAt.eq(0)),
            )
            .all(&app.state.database)
            .await?;

        let base_string = "".to_string();
        for setting in global_settings.iter() {
            //global.insert(setting.setting)
            let name = names.get(&setting.setting).unwrap_or(&base_string).clone();
            global.insert(name.clone(), (setting.clone(), timestamp));
        }

        Ok(ApplicationSettings {
            application: app.clone(),
            base,
            global,
            names,
            user,
        })
    }

    /// refresh the current settings in the data structure with the refreshed values
    /// todo: **This is non functional right now.** This is more so for user settings
    /// good for long running processes that have lots of user interaction and configuration
    pub async fn refresh(&mut self) -> &mut Self {
        // todo: this needs to be filled out

        self
    }

    /// get the setting value that is already cached and loaded from our database.
    /// if a user setting does not exist for the targeted user. The global setting will be passed through instead
    /// if there is no global setting. **None** will be returned as an Option value.
    pub fn get(&self, app_type: ApplicationSettingType, name: &str, user: Option<String>) -> Option<String> {
        match app_type {
            ApplicationSettingType::Global => self.global.get(name).map(|(setting, _)| setting.value.clone()),
            ApplicationSettingType::User => {
                let user = user
                    .as_ref()
                    .map_or("".to_string(), |targeted_user| targeted_user.clone());
                self.user
                    .get(&(user, name.to_string()))
                    .map_or(self.get(ApplicationSettingType::Global, name, None), |(setting, _)| {
                        Some(setting.value.clone())
                    })
            }
        }
    }

    /// update a setting in our cache / database.
    /// in the event that this setting does not exist. It will auto create it in the database accordingly
    pub async fn set(
        &mut self,
        setting_type: ApplicationSettingType,
        name: &str,
        value: &str,
        user: Option<String>,
    ) -> anyhow::Result<JoinHandle<()>> {
        let timestamp = unix_timestamp();
        let setting_model = if let Some((model, timestamp)) = self.base.get(name) {
            Some(model.clone())
        } else {
            let seed = format!("{}|{}|{}", timestamp, self.application.record.id, name);
            let hash = format!("{:x}", md5::compute(seed));

            // create model
            let core_model = application_settings::ActiveModel {
                id: ActiveValue::NotSet,
                application: ActiveValue::Set(self.application.record.id),
                hash: ActiveValue::Set(hash),
                name: ActiveValue::Set(name.to_string()),
                created_at: ActiveValue::Set(timestamp),
                updated_at: ActiveValue::Set(0),
                deleted_at: ActiveValue::Set(0),
            };

            // insert and fetch
            let insert = application_settings::Entity::insert(core_model)
                .exec(&self.application.state.database)
                .await?;
            application_settings::Entity::find_by_id(insert.last_insert_id)
                .one(&self.application.state.database)
                .await?
        };

        // map the setting out out of the setting model if possible
        let setting_id = setting_model.as_ref().map_or(0, |m| m.id);

        // create if neccessary
        match setting_type {
            ApplicationSettingType::Global => {
                let do_create = if let Some((model, _)) = self.global.get(name) {
                    model.id == 0
                } else {
                    true
                };

                if do_create {
                    let seed = format!("{}|{}|global|{}", timestamp, self.application.record.id, name);
                    let hash = format!("{:x}", md5::compute(seed));
                    let active = application_global_settings::ActiveModel {
                        id: ActiveValue::NotSet,
                        application: ActiveValue::Set(self.application.record.id),
                        hash: ActiveValue::Set(hash),
                        setting: ActiveValue::Set(setting_id),
                        value: ActiveValue::Set(value.to_string()),
                        created_at: ActiveValue::Set(timestamp),
                        updated_at: ActiveValue::Set(0),
                        deleted_at: ActiveValue::Set(0),
                    };

                    let insert = application_global_settings::Entity::insert(active)
                        .exec(&self.application.state.database)
                        .await?;
                    let model = application_global_settings::Entity::find_by_id(insert.last_insert_id)
                        .one(&self.application.state.database)
                        .await?;

                    if let Some(model) = model {
                        self.global
                            .entry(name.to_string())
                            .and_modify(|(old_model, model_timestamp)| {
                                *model_timestamp = timestamp;
                                old_model.id = model.id;
                                old_model.hash = model.hash.clone();
                            })
                            .or_insert((model, timestamp));
                    }
                }
            }
            ApplicationSettingType::User => {
                let target_user = user.as_ref().map_or(String::new(), |v| v.clone());
                let target_key = (target_user.clone(), name.to_string());
                let do_create = if let Some((model, _)) = self.user.get(&target_key) {
                    model.id == 0
                } else {
                    true
                };

                if do_create {
                    let seed = format!("{}|{}|user|{}", timestamp, self.application.record.id, name);
                    let hash = format!("{:x}", md5::compute(seed));

                    let active = application_user_settings::ActiveModel {
                        id: ActiveValue::NotSet,
                        application: ActiveValue::Set(self.application.record.id),
                        hash: ActiveValue::Set(hash),
                        hash_user: ActiveValue::Set(target_user.clone()),
                        setting: ActiveValue::Set(setting_id),
                        value: ActiveValue::Set(value.to_string()),
                        created_at: ActiveValue::Set(timestamp),
                        updated_at: ActiveValue::Set(0),
                        deleted_at: ActiveValue::Set(0),
                    };

                    let insert = application_user_settings::Entity::insert(active)
                        .exec(&self.application.state.database)
                        .await?;
                    let model = application_user_settings::Entity::find_by_id(insert.last_insert_id)
                        .one(&self.application.state.database)
                        .await?;

                    if let Some(model) = model {
                        self.user
                            .entry(target_key)
                            .and_modify(|(old_model, model_timestamp)| {
                                *model_timestamp = timestamp;
                                old_model.id = model.id;
                                old_model.hash = model.hash.clone();
                                old_model.hash_user = model.hash_user.clone();
                            })
                            .or_insert((model, timestamp));
                    }
                }
            }
        }

        // update in cache if possible
        match setting_type {
            ApplicationSettingType::Global => {
                self.global
                    .entry(name.to_string())
                    .and_modify(|(model, model_timestamp)| {
                        *model_timestamp = timestamp;
                        model.value = value.to_string();
                        model.updated_at = timestamp;
                    })
                    .or_insert((
                        application_global_settings::Model {
                            id: 0, // not synced
                            application: self.application.record.id,
                            hash: String::new(),
                            setting: setting_id,
                            value: value.to_string(),
                            created_at: timestamp,
                            updated_at: 0,
                            deleted_at: 0,
                        },
                        timestamp,
                    ));
            }
            ApplicationSettingType::User => {
                let target_user = user.as_ref().map_or(String::new(), |v| v.clone());
                let target_key = (target_user.clone(), name.to_string());
                self.user
                    .entry(target_key)
                    .and_modify(|(model, model_timestamp)| {
                        *model_timestamp = timestamp;
                        model.value = value.to_string();
                        model.updated_at = timestamp;
                    })
                    .or_insert((
                        application_user_settings::Model {
                            id: 0,
                            application: self.application.record.id,
                            hash: String::new(),
                            hash_user: target_user.clone(),
                            setting: setting_id,
                            value: value.to_string(),
                            created_at: timestamp,
                            updated_at: 0,
                            deleted_at: 0,
                        },
                        timestamp,
                    ));
            }
        }

        // update in database now, but fire off into its own task
        let db_handle = self.application.state.database.clone();

        let value_clone = value.to_string();
        let handle = match setting_type {
            ApplicationSettingType::Global => {
                let active = self.global.get(name).map(|(m, _)| m.clone());
                tokio::spawn(async move {
                    if let Some(active) = active {
                        let mut active: application_global_settings::ActiveModel = active.into();
                        active.value = ActiveValue::Set(value_clone);
                        active.updated_at = ActiveValue::Set(unix_timestamp());
                        let _ = active.update(&db_handle).await;
                    }
                })
            }
            ApplicationSettingType::User => {
                let target_user = user.as_ref().map_or(String::new(), |v| v.clone());
                let target_key = (target_user.clone(), name.to_string());
                let active = self.user.get(&target_key).map(|(m, _)| m.clone());
                tokio::spawn(async move {
                    if let Some(active) = active {
                        let mut active: application_user_settings::ActiveModel = active.into();
                        active.value = ActiveValue::Set(value_clone);
                        active.updated_at = ActiveValue::Set(unix_timestamp());
                        let _ = active.update(&db_handle).await;
                    }
                })
            }
        };

        Ok(handle)
    }
}
