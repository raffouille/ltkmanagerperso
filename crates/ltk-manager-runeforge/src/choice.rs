//! The post that a publish targets first: the stamped one, the suggested one or a new one.

use serde::{Deserialize, Serialize};

use crate::wire::{OwnedModPost, SuggestionMatch, Suggestions};

/// The site id that a RuneForge distributor stamp carries.
pub const SITE_ID: &str = "runeforge";

/// The site name that a RuneForge distributor stamp carries.
pub const SITE_NAME: &str = "RuneForge";

/// The post that the publish dialog selects when it opens, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Target {
    /// The project stamp names a post of the account.
    Stamped { mod_id: String },
    /// The server suggested a post of the account.
    Suggested {
        mod_id: String,
        matched_by: SuggestionMatch,
    },
    /// No post matches, so the publish creates a new one.
    New,
}

/// The target of a publish, and whether the project stamp names a post the account does not own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TargetChoice {
    /// The post that the dialog selects.
    pub target: Target,
    /// The post that the project stamp names if the account does not own it.
    pub foreign_stamp: Option<String>,
}

/// Chooses the target of a publish.
///
/// Returns the post that `stamp` names if `owned` holds it, else the post of `suggestions`, else
/// a new post. `stamp` is the RuneForge mod id of the project stamp.
#[must_use]
pub fn choose_target(
    stamp: Option<&str>,
    owned: &[OwnedModPost],
    suggestions: &Suggestions,
) -> TargetChoice {
    let owns = |mod_id: &str| owned.iter().any(|post| post.id == mod_id);

    let foreign_stamp = stamp.filter(|mod_id| !owns(mod_id)).map(str::to_owned);

    let target = if let Some(mod_id) = stamp.filter(|mod_id| owns(mod_id)) {
        Target::Stamped {
            mod_id: mod_id.to_owned(),
        }
    } else if let (Some(mod_id), Some(matched_by)) = (&suggestions.mod_id, suggestions.matched_by) {
        Target::Suggested {
            mod_id: mod_id.clone(),
            matched_by,
        }
    } else {
        Target::New
    };

    TargetChoice {
        target,
        foreign_stamp,
    }
}

#[cfg(test)]
mod tests;
