//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.14

use sea_orm::entity::prelude::*;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "application_processes"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Defaul)]
pub struct Model {
    pub id: i64,
    pub application: i64,
    pub hash: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    Application,
    Hash,
    Name,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    Id,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = i64;
    fn auto_increment() -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    ApplicationProcessLogs,
    Applications,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigInteger.def(),
            Self::Application => ColumnType::BigInteger.def(),
            Self::Hash => ColumnType::Char(Some(32u32)).def().unique(),
            Self::Name => ColumnType::String(Some(255u32)).def(),
            Self::CreatedAt => ColumnType::BigInteger.def(),
            Self::UpdatedAt => ColumnType::BigInteger.def(),
            Self::DeletedAt => ColumnType::BigInteger.def(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::ApplicationProcessLogs => Entity::has_many(super::application_process_logs::Entity).into(),
            Self::Applications => Entity::belongs_to(super::applications::Entity)
                .from(Column::Application)
                .to(super::applications::Column::Id)
                .into(),
        }
    }
}

impl Related<super::application_process_logs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ApplicationProcessLogs.def()
    }
}

impl Related<super::applications::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Applications.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
