use assert_matches::assert_matches;

use super::*;

const TOKEN: &str = "rf_abcdEFGH0123-_ijklmnopqrstuvwxyzABCDEFGHIJKLMNO";

#[test]
fn api_token_new_accepts_url_safe_token() {
    let token = ApiToken::new(TOKEN).unwrap();

    assert_eq!(token.expose(), TOKEN);
}

#[test]
fn api_token_new_trims_whitespace() {
    let token = ApiToken::new(&format!("  {TOKEN}\n")).unwrap();

    assert_eq!(token.expose(), TOKEN);
}

#[test]
fn api_token_new_fails_without_prefix() {
    assert_matches!(
        ApiToken::new("abcdEFGH0123"),
        Err(TokenError::MissingPrefix)
    );
}

#[test]
fn api_token_new_fails_if_body_is_short() {
    assert_matches!(ApiToken::new("rf_abc"), Err(TokenError::TooShort));
}

#[test]
fn api_token_new_fails_on_character_outside_alphabet() {
    assert_matches!(
        ApiToken::new("rf_abcdEFGH0123+/"),
        Err(TokenError::InvalidCharacter)
    );
}

#[test]
fn api_token_display_prefix_returns_eight_characters_after_prefix() {
    assert_eq!(ApiToken::new(TOKEN).unwrap().display_prefix(), "abcdEFGH");
}

#[test]
fn api_token_debug_and_display_print_prefix_only() {
    let token = ApiToken::new(TOKEN).unwrap();

    assert_eq!(token.to_string(), "rf_abcdEFGH...");
    assert_eq!(format!("{token:?}"), "ApiToken(rf_abcdEFGH...)");
}

#[test]
fn token_store_load_returns_saved_token() {
    let store = TokenStore::new(keyring_core::mock::Store::new().unwrap());
    let token = ApiToken::new(TOKEN).unwrap();

    store.save(&token).unwrap();

    assert_eq!(store.load().unwrap(), Some(token));
}

#[test]
fn token_store_load_returns_none_after_delete() {
    let store = TokenStore::new(keyring_core::mock::Store::new().unwrap());
    store.save(&ApiToken::new(TOKEN).unwrap()).unwrap();

    store.delete().unwrap();

    assert_eq!(store.load().unwrap(), None);
}

#[test]
fn token_store_delete_succeeds_without_token() {
    let store = TokenStore::new(keyring_core::mock::Store::new().unwrap());

    store.delete().unwrap();
}
