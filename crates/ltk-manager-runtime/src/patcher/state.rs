//! Lifecycle state shared between a patching session and its callers.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread::JoinHandle;

use serde::{Deserialize, Serialize};

use ltk_manager_base::config::BuiltinModSettings;

use super::refresh::OverlayRefresh;
use crate::launcher::champ_select::PickedSkin;

/// Current phase of the patcher lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum PatcherPhase {
    Idle,
    Building,
    Patching,
}

/// Patcher configuration stashed so hot-reload can restart with the same
/// options.
#[derive(Debug, Clone)]
pub struct StoredPatcherConfig {
    pub flags: Option<u64>,
    pub workshop_projects: Option<Vec<String>>,
    /// The layers each workshop project is tested with, by project path. A
    /// project missing from the map is tested with every layer.
    pub workshop_layers: Option<HashMap<String, Vec<String>>>,
}

impl StoredPatcherConfig {
    /// What a session started from this config covers.
    pub fn origin(&self) -> SessionOrigin {
        match self.workshop_projects.as_deref() {
            Some(projects) if !projects.is_empty() => SessionOrigin::Workshop {
                projects: projects.to_vec(),
            },
            _ => SessionOrigin::Library,
        }
    }
}

/// What a patching session was started for, and what it covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum SessionOrigin {
    /// The library's enabled mods.
    Library,
    /// A workshop test over these project directories.
    Workshop {
        /// Absolute paths to the project directories under test.
        projects: Vec<String>,
    },
}

impl SessionOrigin {
    /// Whether the session is a workshop test rather than a library run.
    pub fn is_workshop(&self) -> bool {
        matches!(self, Self::Workshop { .. })
    }
}

/// A patching session, from the moment it is asked for until the thread exits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PatcherSession {
    /// What the session was started for.
    pub origin: SessionOrigin,
    /// The overlay root the session patches against, with a trailing separator.
    ///
    /// `None` until the build phase produces one.
    pub overlay_prefix: Option<String>,
}

pub struct PatcherStateInner {
    /// Flag to signal the patcher thread to stop.
    pub stop_flag: Arc<AtomicBool>,
    /// A library edit the running session's overlay has not caught up with.
    pub overlay_refresh: Arc<OverlayRefresh>,
    /// Handle to the patcher thread.
    pub thread_handle: Option<JoinHandle<()>>,
    /// The session in flight. `None` while idle.
    pub session: Option<PatcherSession>,
    /// Current phase of the patcher lifecycle.
    pub phase: PatcherPhase,
    /// Last patcher config used, for hot-reload.
    pub last_config: Option<StoredPatcherConfig>,
    /// The built-in mod settings saved since the session started, which its next build reads.
    pub builtin_mods: Option<BuiltinModSettings>,
    /// The skin past the base that the local player last picked in champion select, which the
    /// next build keeps.
    pub picked_skin: Option<PickedSkin>,
}

impl PatcherStateInner {
    pub fn new() -> Self {
        Self {
            stop_flag: Arc::new(AtomicBool::new(false)),
            overlay_refresh: Arc::default(),
            thread_handle: None,
            session: None,
            phase: PatcherPhase::Idle,
            last_config: None,
            builtin_mods: None,
            picked_skin: None,
        }
    }

    pub fn is_running(&self) -> bool {
        self.thread_handle
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }

    /// Open a session and enter the build phase.
    pub fn begin_session(&mut self, origin: SessionOrigin) {
        self.phase = PatcherPhase::Building;
        self.builtin_mods = None;
        self.picked_skin = None;
        self.session = Some(PatcherSession {
            origin,
            overlay_prefix: None,
        });
    }

    /// Ask the running session to rebuild its overlay from the library. Idle, it
    /// does nothing, since a start builds from the library anyway.
    pub fn request_overlay_refresh(&self) {
        if self.is_running() {
            self.overlay_refresh.request();
        }
    }

    /// Hand the running session the built-in mod settings just saved, and ask it to rebuild if
    /// they changed. Idle, it does nothing, since a start reads the saved settings.
    pub fn update_builtin_mods(&mut self, settings: &BuiltinModSettings) {
        if self.is_running() && self.builtin_mods.as_ref() != Some(settings) {
            self.builtin_mods = Some(settings.clone());
            self.overlay_refresh.request();
        }
    }

    /// Record the skin picked in champion select, `None` for a base skin, and ask the running
    /// session to rebuild if it changed.
    pub fn set_picked_skin(&mut self, pick: Option<PickedSkin>) {
        if self.is_running() && self.picked_skin != pick {
            tracing::info!("Champion select pick changed to {pick:?}, rebuilding the overlay");
            self.picked_skin = pick;
            self.overlay_refresh.request();
        }
    }

    /// Return to the build phase to rebuild a running session's overlay.
    pub fn resume_building(&mut self) {
        self.phase = PatcherPhase::Building;
    }

    /// Enter the patching phase against the overlay the build produced.
    pub fn enter_patching(&mut self, overlay_prefix: String) {
        self.phase = PatcherPhase::Patching;
        if let Some(session) = self.session.as_mut() {
            session.overlay_prefix = Some(overlay_prefix);
        }
    }

    /// Close the session and return to idle.
    pub fn end_session(&mut self) {
        self.phase = PatcherPhase::Idle;
        self.session = None;
    }
}

impl Default for PatcherStateInner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patcher_state_inner_defaults_to_idle() {
        let inner = PatcherStateInner::new();
        assert_eq!(inner.phase, PatcherPhase::Idle);
        assert!(inner.thread_handle.is_none());
        assert!(inner.session.is_none());
    }

    #[test]
    fn is_running_false_when_no_thread() {
        let inner = PatcherStateInner::new();
        assert!(!inner.is_running());
    }

    #[test]
    fn an_idle_patcher_ignores_a_refresh_request() {
        let inner = PatcherStateInner::new();
        inner.request_overlay_refresh();
        assert!(
            !inner
                .overlay_refresh
                .is_due(std::time::Instant::now() + std::time::Duration::from_secs(60))
        );
    }

    #[test]
    fn patcher_phase_serialization() {
        assert_eq!(
            serde_json::to_string(&PatcherPhase::Idle).unwrap(),
            "\"idle\""
        );
        assert_eq!(
            serde_json::to_string(&PatcherPhase::Building).unwrap(),
            "\"building\""
        );
        assert_eq!(
            serde_json::to_string(&PatcherPhase::Patching).unwrap(),
            "\"patching\""
        );
    }
}
