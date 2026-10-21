//! Unit tests for how a champion select pick is read.

use super::*;

const SESSION: &str = r#"{
    "localPlayerCellId": 2,
    "myTeam": [
        { "cellId": 1, "championId": 222, "selectedSkinId": 222004 },
        { "cellId": 2, "championId": 145, "selectedSkinId": 145014 }
    ],
    "timer": { "phase": "FINALIZATION" }
}"#;

#[test]
fn local_pick_reads_the_local_player_s_champion_and_skin() {
    assert_eq!(local_pick(SESSION), Some((145, 145014)));
}

#[test]
fn local_pick_is_none_before_the_player_has_a_champion() {
    let session = r#"{
        "localPlayerCellId": 0,
        "myTeam": [{ "cellId": 0, "championId": 0, "selectedSkinId": 0 }]
    }"#;

    assert_eq!(local_pick(session), None);
}

#[test]
fn local_pick_is_none_for_a_body_that_is_not_a_session() {
    assert_eq!(local_pick(r#"{ "errorCode": "RPC_ERROR" }"#), None);
}

#[test]
fn skin_number_is_the_skin_id_past_the_champion_s_thousands() {
    assert_eq!(skin_number(145, 145014), 14);
    assert_eq!(skin_number(145, 145000), 0);
}

#[test]
fn skin_number_is_zero_for_a_skin_of_another_champion() {
    assert_eq!(skin_number(145, 222004), 0);
}

#[test]
fn champion_aliases_map_each_id_to_its_alias() {
    let summary = r#"[
        { "id": -1, "alias": "None" },
        { "id": 62, "alias": "MonkeyKing", "name": "Wukong" },
        { "id": 145, "alias": "Kaisa", "name": "Kai'Sa" }
    ]"#;

    assert_eq!(
        champion_aliases(summary),
        HashMap::from([(62, "MonkeyKing".to_owned()), (145, "Kaisa".to_owned())])
    );
}
