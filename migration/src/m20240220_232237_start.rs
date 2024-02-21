use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Application::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Application::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Application::Hash).char_len(32).not_null())
                    .col(ColumnDef::new(Application::HashSecret).char_len(32).not_null())
                    .col(ColumnDef::new(Application::Name).string_len(32).not_null())
                    .col(ColumnDef::new(Application::Host).string_len(255).not_null())
                    .col(
                        ColumnDef::new(Application::CreatedAt)
                            .big_integer()
                            .not_null()
                            .unsigned(),
                    )
                    .col(
                        ColumnDef::new(Application::UpdatedAt)
                            .big_integer()
                            .not_null()
                            .unsigned(),
                    )
                    .col(
                        ColumnDef::new(Application::DeletedAt)
                            .big_integer()
                            .not_null()
                            .unsigned(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ApplicationSetting::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ApplicationSetting::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ApplicationSetting::Hash).char_len(32).not_null())
                    .col(ColumnDef::new(ApplicationSetting::Name).string().not_null())
                    .col(
                        ColumnDef::new(ApplicationSetting::CreatedAt)
                            .big_integer()
                            .not_null()
                            .unsigned(),
                    )
                    .col(
                        ColumnDef::new(ApplicationSetting::UpdatedAt)
                            .big_integer()
                            .not_null()
                            .unsigned(),
                    )
                    .col(
                        ColumnDef::new(ApplicationSetting::DeletedAt)
                            .big_integer()
                            .not_null()
                            .unsigned(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ApplicationGlobalSetting::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ApplicationGlobalSetting::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ApplicationGlobalSetting::Hash).char_len(32).not_null())
                    .col(ColumnDef::new(ApplicationGlobalSetting::Setting).integer().not_null())
                    .col(
                        ColumnDef::new(ApplicationGlobalSetting::Value)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(ApplicationGlobalSetting::CreatedAt)
                            .big_integer()
                            .not_null()
                            .unsigned(),
                    )
                    .col(
                        ColumnDef::new(ApplicationGlobalSetting::UpdatedAt)
                            .big_integer()
                            .not_null()
                            .unsigned(),
                    )
                    .col(
                        ColumnDef::new(ApplicationGlobalSetting::DeletedAt)
                            .big_integer()
                            .not_null()
                            .unsigned(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ApplicationUserSettings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ApplicationUserSettings::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ApplicationUserSettings::Hash).char_len(32).not_null())
                    .col(ColumnDef::new(ApplicationUserSettings::Setting).integer().not_null())
                    .col(
                        ColumnDef::new(ApplicationUserSettings::Value)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(ApplicationUserSettings::CreatedAt)
                            .big_integer()
                            .not_null()
                            .unsigned(),
                    )
                    .col(
                        ColumnDef::new(ApplicationUserSettings::UpdatedAt)
                            .big_integer()
                            .not_null()
                            .unsigned(),
                    )
                    .col(
                        ColumnDef::new(ApplicationUserSettings::DeletedAt)
                            .big_integer()
                            .not_null()
                            .unsigned(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Application::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(ApplicationSetting::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(ApplicationGlobalSetting::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(ApplicationUserSettings::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
#[sea_orm(rename = "applications")]
enum Application {
    Table,
    Id,
    Hash,
    HashSecret,
    Name,
    Host,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
#[sea_orm(rename = "application_settings")]
enum ApplicationSetting {
    Table,
    Id,
    Hash,
    Name,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
#[sea_orm(rename = "application_global_settings")]
enum ApplicationGlobalSetting {
    Table,
    Id,
    Hash,
    Setting,
    Value,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
#[sea_orm(rename = "application_user_settings")]
enum ApplicationUserSettings {
    Table,
    Id,
    Hash,
    Setting,
    Value,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}
