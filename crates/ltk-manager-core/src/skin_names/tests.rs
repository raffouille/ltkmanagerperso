//! Unit tests for how CommunityDragon's champion data is read into skin names.

use super::*;

const KAISA: &str = r#"{
    "id": 145,
    "skins": [
        { "id": 145000, "name": "Kai'Sa" },
        {
            "id": 145001,
            "name": "Kai'Sa ange exterminateur",
            "chromas": [{ "id": 145003, "name": "Kai'Sa ange exterminateur (rubis)" }]
        },
        {
            "id": 145026,
            "name": "Kai'Sa céleste",
            "questSkinInfo": { "tiers": [{ "id": 145027, "name": "Kai'Sa céleste (forme 2)" }] }
        }
    ]
}"#;

#[test]
fn skin_numbers_name_each_skin_chroma_and_tier_by_number() {
    let champion: Champion = serde_json::from_str(KAISA).unwrap();

    let names = skin_numbers(145, champion);

    assert_eq!(names[&0], "Kai'Sa");
    assert_eq!(names[&1], "Kai'Sa ange exterminateur");
    assert_eq!(names[&3], "Kai'Sa ange exterminateur (rubis)");
    assert_eq!(names[&27], "Kai'Sa céleste (forme 2)");
    assert_eq!(names.len(), 5);
}

#[test]
fn locale_dir_lowercases_and_serves_english_from_default() {
    assert_eq!(locale_dir("fr_FR"), "fr_fr");
    assert_eq!(locale_dir("en_US"), "default");
}
