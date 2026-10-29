use super::*;

fn post(id: &str) -> OwnedModPost {
    OwnedModPost {
        id: id.to_owned(),
        name: format!("Post {id}"),
        visibility: "private".to_owned(),
        is_published: false,
        updated_at: "2026-10-27T00:00:00Z".to_owned(),
        latest_release: None,
    }
}

fn suggested(mod_id: Option<&str>, matched_by: Option<SuggestionMatch>) -> Suggestions {
    Suggestions {
        mod_id: mod_id.map(str::to_owned),
        matched_by,
        next_tag: None,
    }
}

#[test]
fn choose_target_returns_stamped_post_if_owned() {
    let choice = choose_target(
        Some("a"),
        &[post("a"), post("b")],
        &suggested(Some("b"), Some(SuggestionMatch::Name)),
    );

    assert_eq!(
        choice.target,
        Target::Stamped {
            mod_id: "a".to_owned()
        }
    );
    assert_eq!(choice.foreign_stamp, None);
}

#[test]
fn choose_target_falls_back_to_suggestion_if_stamp_is_not_owned() {
    let choice = choose_target(
        Some("x"),
        &[post("b")],
        &suggested(Some("b"), Some(SuggestionMatch::Name)),
    );

    assert_eq!(
        choice.target,
        Target::Suggested {
            mod_id: "b".to_owned(),
            matched_by: SuggestionMatch::Name
        }
    );
    assert_eq!(choice.foreign_stamp.as_deref(), Some("x"));
}

#[test]
fn choose_target_returns_suggestion_by_name_without_stamp() {
    let choice = choose_target(
        None,
        &[post("b")],
        &suggested(Some("b"), Some(SuggestionMatch::Name)),
    );

    assert_eq!(
        choice.target,
        Target::Suggested {
            mod_id: "b".to_owned(),
            matched_by: SuggestionMatch::Name
        }
    );
}

#[test]
fn choose_target_returns_new_if_nothing_matches() {
    let choice = choose_target(None, &[post("b")], &suggested(None, None));

    assert_eq!(choice.target, Target::New);
    assert_eq!(choice.foreign_stamp, None);
}
