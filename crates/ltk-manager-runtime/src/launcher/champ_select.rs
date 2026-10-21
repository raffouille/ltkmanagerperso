//! The skin the local player picks in the League client's champion select.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;

use super::reload::{Lockfile, read_lockfile};

/// How long one request to the League client may take.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

/// A skin the local player picked, by its champion's alias and its number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickedSkin {
    /// The champion's alias, such as `Kaisa`, which its archive is named after.
    pub champion: String,
    /// The skin's number within the champion, `0` for the base skin.
    pub skin: u32,
}

/// Reads champion select from the League client installed at one path.
pub struct ChampSelectReader {
    league_path: PathBuf,
    client: Client,
    /// Each champion's alias by its numeric id, read from the client on the first pick.
    aliases: HashMap<i64, String>,
}

impl ChampSelectReader {
    /// A reader of the client installed at `league_path`. Returns `None` if the HTTP client
    /// cannot be built.
    pub fn new(league_path: PathBuf) -> Option<Self> {
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .inspect_err(|e| tracing::warn!("Champion select: cannot build the HTTP client: {e}"))
            .ok()?;

        Some(Self {
            league_path,
            client,
            aliases: HashMap::new(),
        })
    }

    /// Returns the skin the local player holds in champion select.
    ///
    /// Returns `None` outside champion select, before the player has a champion, or when the
    /// client does not answer.
    pub fn pick(&mut self) -> Option<PickedSkin> {
        let lockfile = read_lockfile(&self.league_path)?;
        let session = self.get(&lockfile, "/lol-champ-select/v1/session")?;
        let (champion_id, skin_id) = local_pick(&session)?;

        if !self.aliases.contains_key(&champion_id) {
            let summary = self.get(&lockfile, "/lol-game-data/assets/v1/champion-summary.json")?;
            self.aliases = champion_aliases(&summary);
        }

        Some(PickedSkin {
            champion: self.aliases.get(&champion_id)?.clone(),
            skin: skin_number(champion_id, skin_id),
        })
    }

    /// Reads the body the client answers `path` with. Returns `None` if the request fails or
    /// the status is not a success.
    fn get(&self, lockfile: &Lockfile, path: &str) -> Option<String> {
        let url = format!("https://127.0.0.1:{}{}", lockfile.port, path);
        let response = self
            .client
            .get(&url)
            .basic_auth("riot", Some(&lockfile.password))
            .send()
            .ok()?;

        if !response.status().is_success() {
            return None;
        }
        response.text().ok()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    local_player_cell_id: i64,
    #[serde(default)]
    my_team: Vec<Member>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Member {
    cell_id: i64,
    #[serde(default)]
    champion_id: i64,
    #[serde(default)]
    selected_skin_id: i64,
}

#[derive(Deserialize)]
struct ChampionSummary {
    id: i64,
    alias: String,
}

/// The champion id and skin id of the local player in the champion select `session` body.
///
/// Returns `None` if the body does not parse or the player has no champion yet.
fn local_pick(session: &str) -> Option<(i64, i64)> {
    let session: Session = serde_json::from_str(session).ok()?;
    let member = session
        .my_team
        .into_iter()
        .find(|member| member.cell_id == session.local_player_cell_id)?;

    (member.champion_id > 0).then_some((member.champion_id, member.selected_skin_id))
}

/// Each champion's alias by its numeric id, out of the client's `champion-summary.json` body.
fn champion_aliases(summary: &str) -> HashMap<i64, String> {
    serde_json::from_str::<Vec<ChampionSummary>>(summary)
        .unwrap_or_default()
        .into_iter()
        .filter(|champion| champion.id > 0)
        .map(|champion| (champion.id, champion.alias))
        .collect()
}

/// The number of `skin_id` within `champion_id`, whose skin ids are the champion id times 1000
/// plus the number. Returns `0` for a skin id of another champion.
fn skin_number(champion_id: i64, skin_id: i64) -> u32 {
    if skin_id / 1000 != champion_id {
        return 0;
    }
    u32::try_from(skin_id % 1000).unwrap_or(0)
}

#[cfg(test)]
mod tests;
