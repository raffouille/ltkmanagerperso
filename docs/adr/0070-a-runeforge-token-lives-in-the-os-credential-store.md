# ADR-0070: A RuneForge token lives in the OS credential store

- **Status:** Accepted (2026-10-27)
- **Date:** 2026-10-27
- **Crates:** `ltk-manager-runeforge`, where `TokenStore` keeps the token and `ApiToken` holds it,
  and `src-tauri`, where `services::runeforge` reads it
- **Related:** `specs/019-runeforge-publishing/spec.md`, "Publishing to RuneForge" in
  `docs/ux/WORKSHOP.md`

## Context and problem statement

Publishing to RuneForge authenticates with a personal API token, `rf_` and 43 URL-safe base64
characters. The token acts as the account within its scopes, and the `publish` scope makes a post
public. It is the first secret the app keeps for the user.

Two other secrets are in plain files: `Settings.telemetry_secret` in `settings.json` and the bearer
token of the patcher delivery in `delivery.json`. `settings.json` is exported by the settings
backup and attached to support requests, so a secret in it leaves the machine with the file.

## Decision

**The token is in the OS credential store.** `TokenStore` writes it through `keyring-core` and
`windows-native-keyring-store` to the Windows Credential Manager, under the service
`ltk-manager/runeforge` and the account `default`. Another platform has no store wired in, and a
connect there fails with `RuneforgeError::Credential`.

**`settings.json` keeps only what is safe to read.** `Settings.runeforge` holds the user id, the
user name, the display name, the token prefix and the scopes. Only the runeforge commands write
it, and `save_settings` keeps the stored value. A settings backup leaves it out, and a restore keeps
the current one.

**The token never leaves the backend.** `ApiToken` implements neither `Serialize` nor a `Debug` or
`Display` that prints the secret. Both print `rf_` and the 8 characters that the token page lists.
The frontend sends the pasted text once to `runeforge_connect` and never reads it back.

**The token goes to the API host only.** `Client` adds the `Authorization` header to the requests it
builds under the base URL. The PUT of the package bytes goes to a presigned storage URL without it,
and a redirect is followed only to the host that the request was sent to.

**A refused token stays stored.** A 401 marks the connection rejected for the session and the
Settings section offers Reconnect. The token is deleted only by Disconnect or replaced by a connect.

## Consequences

- **Positive:** a copied `settings.json`, a settings backup or a support bundle holds no token.
- **Positive:** the Credential Manager protects the token with the Windows account of the user.
- **Negative:** a token revoked on the site stays on the machine until the user disconnects.
- **Negative:** `telemetry_secret` and the delivery bearer token stay in plain files. Moving them is
  a separate change.
