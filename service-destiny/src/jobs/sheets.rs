use google_sheets4::hyper::Client;
use google_sheets4::oauth2::{self};
use google_sheets4::{hyper, hyper_rustls, Sheets};
use google_sheets4::{
    hyper::client::HttpConnector, hyper_rustls::HttpsConnector, oauth2::authenticator::Authenticator,
};
use levelcrush::{anyhow, project_str, tracing};
use lib_destiny::app::state::AppState;
use lib_destiny::env::{AppVariable, Env};
use std::collections::HashMap;

const GOOGLE_CREDENTIALS: &str = project_str!("google_credentials.json");

const SHEET_PLAYER_LIST: &str = "'Player List'";
const SHEET_TEMPLATE_ROSTER: &str = "'[Template] Roster'";

#[derive(Debug, Clone)]
pub struct WorksheetPlayer {
    pub bungie_name: String,
    pub discord_name: String,
    pub discord_id: String,
    pub bungie_membership_id: String,
    pub bungie_platform: String,
}

#[derive(Debug, Clone)]
pub struct WorksheetClan {
    pub name: String,
    pub group_id: i64,
    pub members: Vec<(String, i64)>,
}

#[derive(Clone)]
pub struct MasterWorkbook {
    pub sheet_id: String,
    pub player_list: HashMap<String, WorksheetPlayer>,
    pub clans: HashMap<i64, WorksheetClan>,
    pub google: Sheets<HttpsConnector<HttpConnector>>,
}

impl MasterWorkbook {
    pub async fn get(sheet_id: &str) -> anyhow::Result<MasterWorkbook> {
        tracing::info!("Constructing client");
        let client = hyper::Client::builder().build(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .https_only()
                .enable_http1()
                .enable_http2()
                .build(),
        );

        tracing::info!("Constructing service key");
        let secret = oauth2::read_service_account_key("google_credentials.json").await?;

        tracing::info!("Building authenticating");
        let auth = oauth2::ServiceAccountAuthenticator::with_client(secret, client.clone())
            .build()
            .await?;

        let google = Sheets::new(client.clone(), auth);
        let workbook = MasterWorkbook {
            sheet_id: sheet_id.to_string(),
            player_list: HashMap::new(),
            clans: HashMap::new(),
            google,
        };

        Ok(workbook)
    }

    /// this will populate our Masterworkbook data structure with data from the spreadsheet
    /// this will make 0 api calls to the bungie api
    /// use this function to provide a state that is READ FROM THE SPREADSHEET
    pub async fn load(&mut self) -> anyhow::Result<()> {
        // clear arrays
        self.clans.clear();
        self.player_list.clear();

        let (_, workbook) = self.google.spreadsheets().get(&self.sheet_id).doit().await?;

        // grab all clan sheet names ahead of time
        let mut clan_sheet_names = Vec::new();
        let sheets = workbook.sheets.unwrap_or_default();
        for sheet in sheets.into_iter() {
            if let Some(properties) = sheet.properties {
                let sheet_title = properties.title.unwrap_or_default();
                if sheet_title.contains("[Clan]") {
                    clan_sheet_names.push(sheet_title);
                }
            }
        }

        // player sheet parsing
        // Parse the player sheet and pull all relevant player info where possible
        let player_sheet_range = format!("{SHEET_PLAYER_LIST}!A2:E");
        let (_, player_list_range) = self
            .google
            .spreadsheets()
            .get(&self.sheet_id)
            .add_ranges(&player_sheet_range)
            .include_grid_data(true)
            .doit()
            .await?;

        let sheets = player_list_range.sheets.unwrap_or_default();
        let player_sheet = sheets.first();
        let base_string = String::new();
        if let Some(player_sheet) = player_sheet {
            let data = player_sheet.data.as_ref().expect("Expecting grid data");
            for grid_data in data.iter() {
                let row_data = grid_data.row_data.as_ref().expect("Expecting row data");
                for row in row_data.iter() {
                    if let Some(cell_data) = row.values.as_ref() {
                        let bungie_name_cell = cell_data.get(0);
                        let discord_name_cell = cell_data.get(1);
                        let discord_id_cell = cell_data.get(2);
                        let bungie_membership_cell = cell_data.get(3);
                        let bungie_membership_platform_cell = cell_data.get(4);

                        let bungie_name = if let Some(bungie_name_cell) = bungie_name_cell {
                            bungie_name_cell
                                .formatted_value
                                .as_ref()
                                .unwrap_or(&base_string)
                                .clone()
                        } else {
                            base_string.clone()
                        };

                        let discord_name = if let Some(discord_name_cell) = discord_name_cell {
                            discord_name_cell
                                .formatted_value
                                .as_ref()
                                .unwrap_or(&base_string)
                                .clone()
                        } else {
                            base_string.clone()
                        };

                        let discord_id = if let Some(discord_id_cell) = discord_id_cell {
                            discord_id_cell.formatted_value.as_ref().unwrap_or(&base_string).clone()
                        } else {
                            base_string.clone()
                        };

                        let bungie_membership = if let Some(bungie_membership_cell) = bungie_membership_cell {
                            bungie_membership_cell
                                .formatted_value
                                .as_ref()
                                .unwrap_or(&base_string)
                                .clone()
                        } else {
                            base_string.clone()
                        };

                        let bungie_platform = if let Some(bugnie_platform_cell) = bungie_membership_platform_cell {
                            bugnie_platform_cell
                                .formatted_value
                                .as_ref()
                                .unwrap_or(&base_string)
                                .clone()
                        } else {
                            base_string.clone()
                        };
                        self.player_list
                            .entry(bungie_membership.clone())
                            .and_modify(|r| {
                                *r = WorksheetPlayer {
                                    bungie_name: bungie_name.clone(),
                                    discord_name: discord_name.clone(),
                                    discord_id: discord_id.clone(),
                                    bungie_membership_id: bungie_membership.clone(),
                                    bungie_platform: bungie_platform.clone(),
                                }
                            })
                            .or_insert(WorksheetPlayer {
                                bungie_name: bungie_name.clone(),
                                discord_name: discord_name.clone(),
                                discord_id: discord_id.clone(),
                                bungie_membership_id: bungie_membership.clone(),
                                bungie_platform: bungie_platform.clone(),
                            });
                    }
                }
            }
        }

        // now parse the clan sheets
        let mut clan_sheet_request = self.google.spreadsheets().get(&self.sheet_id);
        for clan_sheet in clan_sheet_names.iter() {
            let info_range = format!("{clan_sheet}!B1:B3");
            let roster_range = format!("{clan_sheet}!A6:B");
            clan_sheet_request = clan_sheet_request.add_ranges(&info_range).add_ranges(&roster_range);
        }

        let (_, clan_spreadsheet) = clan_sheet_request.include_grid_data(true).doit().await?;
        if let Some(clan_sheets) = clan_spreadsheet.sheets {
            for sheet in clan_sheets.iter() {
                let mut clan_name = None;
                let mut clan_group_id = None;
                let mut clan_total_members = None;
                let mut clan_members = Vec::new();
                let data = sheet.data.as_ref().expect("Expecting grid data");
                for grid_data in data.iter() {
                    let row_data = grid_data.row_data.as_ref().expect("Expecting row data");
                    for row in row_data.iter() {
                        // parse the row here
                        let mut txt_values = Vec::new();
                        if let Some(cell_data) = row.values.as_ref() {
                            txt_values.extend(
                                cell_data
                                    .iter()
                                    .map(|v| v.formatted_value.as_ref().unwrap_or(&base_string).clone())
                                    .collect::<Vec<String>>(),
                            );
                        }

                        if clan_name.is_none() {
                            clan_name = Some(txt_values.first().unwrap_or(&base_string).clone());
                        } else if clan_group_id.is_none() {
                            clan_group_id = Some(
                                txt_values
                                    .first()
                                    .unwrap_or(&base_string)
                                    .clone()
                                    .parse::<i64>()
                                    .unwrap_or_default(),
                            );
                        } else if clan_total_members.is_none() {
                            clan_total_members = Some(txt_values.first().unwrap_or(&base_string).clone());
                        } else {
                            clan_members.push((
                                txt_values.first().unwrap_or(&base_string).clone(),
                                txt_values
                                    .last()
                                    .unwrap_or(&base_string)
                                    .clone()
                                    .parse::<i64>()
                                    .unwrap_or(0),
                            ));
                        }
                    }
                }

                // track
                let clan_id = clan_group_id.unwrap_or_default();
                self.clans.insert(
                    clan_id,
                    WorksheetClan {
                        name: clan_name.unwrap_or_default(),
                        group_id: clan_id,
                        members: clan_members,
                    },
                );
            }
        }

        // clans

        Ok(())
    }

    /// based off information already provided sync using the api
    pub async fn api_sync(&mut self, env: &Env) -> anyhow::Result<()> {
        let mut clan_group_id_strings = Vec::new();
        let mut clan_group_ids = Vec::new();
        for (clan_id, _) in self.clans.iter() {
            clan_group_id_strings.push(clan_id.to_string());
            clan_group_ids.push(*clan_id);
        }

        tracing::info!("Getting latest clan info based off spreadsheet clans");
        lib_destiny::jobs::clan::info(&clan_group_id_strings, env).await?;

        tracing::info!("Getting lastest clan roster info based off spreadsheet clans");
        lib_destiny::jobs::clan::roster(&clan_group_id_strings, env).await?;

        tracing::info!("Marking spreadsheet clans as network");
        lib_destiny::jobs::clan::make_network(&clan_group_id_strings, env).await?;

        tracing::info!("Syncing clan info and roster to local database");
        let mut app_state = AppState::new(env).await;
        for clan_id in clan_group_ids.iter() {
            let clan_info = lib_destiny::app::clan::get(*clan_id, &mut app_state).await;
            let clan_roster = lib_destiny::app::clan::get_roster(*clan_id, &mut app_state).await;

            tracing::info!("Syncing latest {clan_id} info to workbook");
            if let Some(clan_info) = clan_info {
                self.clans.entry(*clan_id).and_modify(|clan| {
                    clan.name = clan_info.name.clone();
                });
            }

            tracing::info!("Syncing latest {clan_id} roster to workbook");
            self.clans.entry(*clan_id).and_modify(|clan| {
                clan.members.clear();
                for member in clan_roster.iter() {
                    let membership_id = member.membership_id.to_string();
                    self.player_list
                        .entry(membership_id.clone())
                        .and_modify(|m| {
                            m.bungie_name = member.display_name_global.clone();
                            m.bungie_platform = member.platform.to_string();
                        })
                        .or_insert(WorksheetPlayer {
                            bungie_membership_id: membership_id.clone(),
                            bungie_name: member.display_name_global.clone(),
                            discord_id: String::new(),
                            discord_name: String::new(),
                            bungie_platform: member.platform.to_string(),
                        });
                    clan.members
                        .push((member.display_name_global.clone(), member.clan_group_role));
                }
            });
        }

        Ok(())
    }

    /// take the info from the local workbook and save it to the google spreadsheet
    pub async fn save(&self) -> anyhow::Result<()> {
        //

        Ok(())
    }
}

pub async fn test_job(env: &Env) -> anyhow::Result<()> {
    tracing::info!("Constructing workbook connection");
    let sheet_id = env.get(AppVariable::MasterWorkSheet);
    let mut workbook = MasterWorkbook::get(&sheet_id).await?;

    tracing::info!("Hydrating information");
    workbook.load().await?;

    tracing::info!("Updating from API");
    workbook.api_sync(env).await?;

    tracing::info!("{:?}", workbook.player_list);

    Ok(())
}