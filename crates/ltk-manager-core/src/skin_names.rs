//! Skin names from CommunityDragon's copy of the League client's game data.

use std::collections::HashMap;
use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;

/// Where CommunityDragon serves the client's game data, one directory per locale.
const GAME_DATA_URL: &str =
    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global";

/// The directory CommunityDragon serves English from.
const DEFAULT_LOCALE: &str = "default";

const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Deserialize)]
struct ChampionSummary {
    id: i64,
    alias: String,
}

#[derive(Deserialize)]
struct Champion {
    #[serde(default)]
    skins: Vec<Skin>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Skin {
    id: i64,
    name: String,
    #[serde(default)]
    chromas: Vec<Named>,
    quest_skin_info: Option<QuestSkinInfo>,
}

#[derive(Deserialize)]
struct QuestSkinInfo {
    #[serde(default)]
    tiers: Vec<Named>,
}

#[derive(Deserialize)]
struct Named {
    id: Option<i64>,
    name: Option<String>,
}

/// Reads the name of each skin of `champion`, chromas and tiers included, by skin number.
///
/// `champion` is the alias an archive is named after, such as `Kaisa`, and `locale` the client
/// locale, such as `fr_FR`. Falls back to English if the locale is `None` or cannot be read.
/// Returns an empty map if CommunityDragon cannot be reached or does not list the champion.
pub fn skin_names(champion: &str, locale: Option<&str>) -> HashMap<u32, String> {
    let Ok(client) = Client::builder().timeout(FETCH_TIMEOUT).build() else {
        return HashMap::new();
    };

    let wanted = locale.map(locale_dir);
    let dirs = wanted
        .iter()
        .map(String::as_str)
        .chain([DEFAULT_LOCALE])
        .collect::<Vec<_>>();

    for dir in dirs {
        if let Some(names) = names_in(&client, dir, champion) {
            return names;
        }
    }

    tracing::warn!("Skin names: CommunityDragon lists no skins of {champion}");
    HashMap::new()
}

/// The skin names of `champion` in the locale directory `dir`, or `None` if a read fails.
fn names_in(client: &Client, dir: &str, champion: &str) -> Option<HashMap<u32, String>> {
    let summary: Vec<ChampionSummary> = get(client, &format!("{dir}/v1/champion-summary.json"))?;
    let id = summary
        .into_iter()
        .find(|entry| entry.alias.eq_ignore_ascii_case(champion))?
        .id;

    let champion: Champion = get(client, &format!("{dir}/v1/champions/{id}.json"))?;
    Some(skin_numbers(id, champion))
}

/// Reads `path` under [`GAME_DATA_URL`] as JSON. Returns `None` if the read or the parse fails.
fn get<T: DeserializeOwned>(client: &Client, path: &str) -> Option<T> {
    let response = client
        .get(format!("{GAME_DATA_URL}/{path}"))
        .send()
        .inspect_err(|e| tracing::warn!("Skin names: cannot read {path}: {e}"))
        .ok()?;

    if !response.status().is_success() {
        tracing::debug!("Skin names: {path} answered {}", response.status());
        return None;
    }
    response.json().ok()
}

/// The CommunityDragon directory of the client locale `locale`, lowercased, with English in
/// [`DEFAULT_LOCALE`].
fn locale_dir(locale: &str) -> String {
    let dir = locale.to_ascii_lowercase();
    if dir == "en_us" {
        return DEFAULT_LOCALE.to_owned();
    }
    dir
}

/// Each skin, chroma and tier name of `champion`, by its number past `champion_id` times 1000.
fn skin_numbers(champion_id: i64, champion: Champion) -> HashMap<u32, String> {
    let mut names = HashMap::new();
    let mut add = |id: i64, name: String| {
        if id / 1000 == champion_id
            && let Ok(number) = u32::try_from(id % 1000)
        {
            names.insert(number, name);
        }
    };

    for skin in champion.skins {
        let variants = skin
            .chromas
            .into_iter()
            .chain(skin.quest_skin_info.into_iter().flat_map(|info| info.tiers));
        for variant in variants {
            if let (Some(id), Some(name)) = (variant.id, variant.name) {
                add(id, name);
            }
        }
        add(skin.id, skin.name);
    }
    names
}

#[cfg(test)]
mod tests;
