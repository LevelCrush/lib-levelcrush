use std::collections::HashMap;

use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter};

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
    user: HashMap<String, (application_user_settings::Model, UnixTimestamp)>,
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

    /// get the setting stored in our database for this application
    pub async fn get(&self, app_type: ApplicationSettingType, name: &str, user: Option<String>) -> Option<String> {
        match app_type {
            ApplicationSettingType::Global => self.global.get(name).map(|(setting, _)| setting.value.clone()),
            ApplicationSettingType::User => {
                let user = user
                    .as_ref()
                    .map_or("".to_string(), |targeted_user| targeted_user.clone());
                self.user.get(&user).map(|(setting, _)| setting.value.clone())
            }
        }
    }
}
