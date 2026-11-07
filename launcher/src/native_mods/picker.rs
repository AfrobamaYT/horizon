//! Structural rebuild requirements, independent of a player's saved build preference.
use super::{Profile, SourceReason};

/// How a native mod can reach the game.
#[derive(Clone, Debug, PartialEq)]
pub enum Route {
    /// Compiles against the prebuilt game without rebuilding it.
    Prebuilt,
    /// Requires a full game build for this structural reason.
    Game { reason: SourceReason },
    /// Invalid project input; rebuilding the game cannot repair it.
    Invalid { reason: String },
}

/// Per-mod structural routes and a separately probed release/computer limitation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Routes {
    pub mods: Vec<(String, Route)>,
    pub unavailable: Option<SourceReason>,
}

impl Routes {
    /// Rebuild requirements, excluding retryable download failures and malformed projects.
    pub fn rebuild_reasons(&self) -> impl Iterator<Item = &SourceReason> {
        self.unavailable
            .iter()
            .chain(self.mods.iter().filter_map(|(_, route)| match route {
                Route::Game { reason } => Some(reason),
                _ => None,
            }))
            .filter(|reason| reason.forces_source())
    }
}

/// Show build modes only for structural rebuilds, folder-loaded mods or Developer mode.
pub fn needs_picker(routes: &Routes, folders: bool, developer_mode: bool) -> bool {
    folders || developer_mode || routes.rebuild_reasons().next().is_some()
}

impl Profile {
    /// A saved choice applies only while its picker is visible; otherwise use the prebuilt route.
    pub fn for_picker(self, routes: &Routes, folders: bool, developer_mode: bool) -> Self {
        if needs_picker(routes, folders, developer_mode) {
            self
        } else {
            Self::Optimized
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_prebuilt_ignores_saved_modes_but_folder_and_developer_choices_apply() {
        let routes = Routes {
            mods: vec![("api_mod".into(), Route::Prebuilt)],
            unavailable: None,
        };
        for saved in Profile::ALL {
            assert!(!needs_picker(&routes, false, false));
            assert_eq!(saved.for_picker(&routes, false, false), Profile::Optimized);
            for (folders, developer) in [(true, false), (false, true), (true, true)] {
                assert!(needs_picker(&routes, folders, developer));
                assert_eq!(saved.for_picker(&routes, folders, developer), saved);
            }
        }
    }

    #[test]
    fn every_structural_reason_exposes_the_picker_and_download_failure_does_not() {
        for reason in [
            SourceReason::Injections { mod_id: "a".into() },
            SourceReason::BuildScript { mod_id: "a".into() },
            SourceReason::Features { mod_id: "a".into() },
            SourceReason::Crate {
                mod_id: "a".into(),
                krate: "game".into(),
            },
            SourceReason::Host,
            SourceReason::NoRelease {
                revision: "revision".into(),
            },
        ] {
            let routes = Routes {
                mods: Vec::new(),
                unavailable: Some(reason),
            };
            assert!(needs_picker(&routes, false, false));
            assert_eq!(
                Profile::Balanced.for_picker(&routes, false, false),
                Profile::Balanced
            );
        }
        let routes = Routes {
            mods: vec![(
                "broken".into(),
                Route::Invalid {
                    reason: "invalid Cargo.toml".into(),
                },
            )],
            unavailable: Some(SourceReason::Download {
                why: "offline".into(),
            }),
        };
        assert!(!needs_picker(&routes, false, false));
        assert_eq!(
            Profile::Developer.for_picker(&routes, false, false),
            Profile::Optimized
        );
    }
}
