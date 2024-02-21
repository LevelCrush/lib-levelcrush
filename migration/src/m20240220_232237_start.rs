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
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ApplicationSettings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ApplicationSettings::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ApplicationSettings::Hash).char_len(32).not_null())
                    .col(ColumnDef::new(ApplicationSettings::Name).string_len(255).not_null())
                    .col(ColumnDef::new(ApplicationSettings::Value).text().not_null().default(""))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Application::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(ApplicationSettings::Table).to_owned())
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
}

#[derive(DeriveIden)]
#[sea_orm(rename = "application_settings")]
enum ApplicationSettings {
    Table,
    Id,
    Hash,
    Name,
    Value,
}
