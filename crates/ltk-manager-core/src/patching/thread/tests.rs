use std::collections::HashMap;

use super::*;

#[test]
fn workshop_tests_take_each_projects_layers() {
    let config = StoredPatcherConfig {
        flags: None,
        workshop_projects: Some(vec!["a".to_owned(), "b".to_owned()]),
        workshop_layers: Some(HashMap::from([("a".to_owned(), vec!["extras".to_owned()])])),
    };

    let tests = workshop_tests(&config);

    assert_eq!(
        tests[0].enabled_layers,
        Some(["extras".to_owned()].into_iter().collect())
    );
    assert_eq!(tests[1].enabled_layers, None);
}

#[test]
fn session_config_reads_saved_built_in_mods_and_keeps_the_picked_skin() {
    let config = Config::default();
    let mut state = PatcherStateInner::new();
    state.builtin_mods = Some(BuiltinModSettings {
        base_skins: BaseSkinsScope::AllChampions,
        kept_skins: [("Jinx".to_owned(), [1].into())].into(),
        ..BuiltinModSettings::default()
    });
    state.picked_skin = Some(ltk_manager_runtime::launcher::champ_select::PickedSkin {
        champion: "Kaisa".to_owned(),
        skin: 14,
    });

    let built = session_config(&config, &state);

    assert_eq!(built.builtin_mods.base_skins, BaseSkinsScope::AllChampions);
    assert_eq!(
        built.builtin_mods.kept_skins,
        [
            ("Jinx".to_owned(), [1].into()),
            ("Kaisa".to_owned(), [14].into())
        ]
        .into()
    );
}

#[test]
fn session_config_is_the_started_config_with_nothing_saved_or_picked() {
    let config = Config::default();

    assert_eq!(
        session_config(&config, &PatcherStateInner::new()).builtin_mods,
        config.builtin_mods
    );
}
