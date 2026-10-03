//! Serializable viewer session — HUD prefs plus per-graph view.
//!
//! Hosts persist the same JSON blob: a desktop file or browser `localStorage`.
//! Node positions are not stored; layout recomputes on open.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const SESSION_VERSION: &str = "hyperviz.session.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewerSession {
    pub version: String,
    #[serde(default)]
    pub prefs: ViewerPrefs,
    /// Views keyed by [`crate::scene::SceneMeta::id`].
    #[serde(default)]
    pub views: BTreeMap<String, ViewerView>,
}

impl Default for ViewerSession {
    fn default() -> Self {
        Self {
            version: SESSION_VERSION.into(),
            prefs: ViewerPrefs::default(),
            views: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ViewerPrefs {
    pub layout: LayoutPrefs,
    pub labels: LabelPrefs,
    pub hulls: HullPrefs,
    pub navigation: NavigationPrefs,
    pub attention: bool,
    /// Kept so older `hyperviz.session.v1` blobs still load. Prefer [`NavigationPrefs::lasso`].
    pub lasso: bool,
    pub window: Option<WindowPrefs>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct NavigationPrefs {
    pub lasso: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutPrefs {
    pub iterations_per_frame: usize,
    pub running: bool,
    pub dt: f32,
    pub damping: f32,
    pub repulsion: f32,
    pub attraction: f32,
    pub theta: f32,
    pub gravity: f32,
    pub ideal_length: f32,
    pub max_tree_depth: usize,
    pub centroid_attraction: f32,
}

impl Default for LayoutPrefs {
    fn default() -> Self {
        Self {
            iterations_per_frame: 4,
            running: true,
            dt: 0.3,
            damping: 0.85,
            repulsion: 500.0,
            attraction: 0.005,
            theta: 0.8,
            gravity: 0.02,
            ideal_length: 30.0,
            max_tree_depth: 14,
            centroid_attraction: 0.008,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LabelPrefs {
    pub vertex_labels: bool,
    pub hyperedge_labels: bool,
    /// `capped`, `selection`, or `all`.
    pub mode: String,
    pub scale: f32,
    pub variation: f32,
}

impl Default for LabelPrefs {
    fn default() -> Self {
        Self {
            vertex_labels: true,
            hyperedge_labels: true,
            mode: "capped".into(),
            scale: 1.25,
            variation: 0.8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HullPrefs {
    pub enabled: bool,
    pub hide_hubs: bool,
    pub wireframe: bool,
    pub opacity: f32,
}

impl Default for HullPrefs {
    fn default() -> Self {
        Self {
            enabled: true,
            hide_hubs: true,
            wireframe: true,
            opacity: 0.28,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowPrefs {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ViewerView {
    pub explorer: Option<ExplorerSession>,
    pub camera: Option<CameraPrefs>,
    pub focus_node_ids: Option<Vec<String>>,
    pub focus_hyperedge_ids: Option<Vec<String>>,
    pub selected_node_ids: Vec<String>,
    pub selected_hyperedge_ids: Vec<String>,
    pub find_query: String,
    pub find_isolated: bool,
    /// Focus neighborhood (⌃⌘F) is on for this graph.
    pub isolated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraPrefs {
    pub focus: [f32; 3],
    pub radius: f32,
    pub yaw: f32,
    pub pitch: f32,
    /// World-space camera translation. Orbit params remain the source of truth.
    #[serde(default)]
    pub position: Option<[f32; 3]>,
}

/// Parse a session blob. Unknown or empty versions are rejected.
pub fn parse_session(raw: &str) -> Option<ViewerSession> {
    let session: ViewerSession = serde_json::from_str(raw).ok()?;
    if session.version != SESSION_VERSION {
        return None;
    }
    Some(session)
}

pub fn session_to_json(session: &ViewerSession) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(session)
}

pub fn view_key(graph_id: &str) -> String {
    if graph_id.is_empty() {
        "_".into()
    } else {
        graph_id.to_string()
    }
}

/// Select the native module canvas or spatial viewer. Auto keeps the spatial viewer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    #[default]
    Auto,
    Spatial,
    Dependencies,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExplorerSnapshot {
    pub center: Option<String>,
    pub target: Option<String>,
    pub options: crate::ScopeOptions,
    pub filters: crate::ModuleFilters,
    pub expanded_ids: Vec<String>,
    pub pan: [f32; 2],
    pub zoom: f32,
}
impl Default for ExplorerSnapshot {
    fn default() -> Self {
        Self {
            center: None,
            target: None,
            options: Default::default(),
            filters: Default::default(),
            expanded_ids: Vec::new(),
            pan: [0., 0.],
            zoom: 1.,
        }
    }
}
impl ExplorerSnapshot {
    pub fn normalize(&mut self) {
        self.options.normalize();
        if self.pan.iter().any(|v| !v.is_finite()) {
            self.pan = [0., 0.];
        }
        self.zoom = if self.zoom.is_finite() {
            self.zoom.clamp(0.1, 4.)
        } else {
            1.
        };
        self.expanded_ids.truncate(1000);
    }
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExplorerSession {
    pub mode: ViewMode,
    pub snapshot: ExplorerSnapshot,
    pub history: Vec<ExplorerSnapshot>,
    pub cursor: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explorer_fields_and_small_opacity_round_trip_without_version_change() {
        let old = parse_session(r#"{"version":"hyperviz.session.v1","views":{"old":{}}}"#).unwrap();
        assert!(old.views["old"].explorer.is_none());
        let mut session = ViewerSession::default();
        session.prefs.hulls.opacity = 0.001;
        let explorer = ExplorerSession {
            mode: ViewMode::Dependencies,
            snapshot: ExplorerSnapshot {
                center: Some("Mathlib.Topology.Basic".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        session.views.insert(
            "mathlib".into(),
            ViewerView {
                explorer: Some(explorer),
                focus_hyperedge_ids: Some(vec!["import:a:b".into()]),
                ..Default::default()
            },
        );
        assert_eq!(
            parse_session(&session_to_json(&session).unwrap()).unwrap(),
            session
        );
    }

    #[test]
    fn round_trip_keeps_prefs_and_per_graph_view() {
        let mut session = ViewerSession::default();
        session.prefs.attention = true;
        session.prefs.navigation.lasso = true;
        session.prefs.layout.repulsion = 900.0;
        session.prefs.window = Some(WindowPrefs {
            width: 1400,
            height: 900,
        });
        session.views.insert(
            "demo-graph".into(),
            ViewerView {
                camera: Some(CameraPrefs {
                    focus: [1.0, 2.0, 3.0],
                    radius: 40.0,
                    yaw: 0.2,
                    pitch: -0.4,
                    position: Some([8.0, 4.0, 12.0]),
                }),
                selected_node_ids: vec!["repo:widgets".into()],
                selected_hyperedge_ids: vec!["he:watch".into()],
                find_query: "watch".into(),
                find_isolated: true,
                isolated: true,
                ..ViewerView::default()
            },
        );
        let raw = session_to_json(&session).expect("json");
        let loaded = parse_session(&raw).expect("parse");
        assert_eq!(loaded, session);
    }

    #[test]
    fn unknown_version_is_rejected() {
        let raw = r#"{"version":"hyperviz.session.v0","prefs":{},"views":{}}"#;
        assert!(parse_session(raw).is_none());
    }

    #[test]
    fn missing_fields_use_defaults() {
        let raw = r#"{"version":"hyperviz.session.v1"}"#;
        let loaded = parse_session(raw).expect("parse");
        assert!(!loaded.prefs.attention);
        assert!(!loaded.prefs.navigation.lasso);
        assert!(loaded.views.is_empty());
        assert_eq!(loaded.prefs.labels.mode, "capped");
    }

    #[test]
    fn legacy_lasso_without_navigation_still_loads() {
        let raw = r#"{"version":"hyperviz.session.v1","prefs":{"lasso":true}}"#;
        let loaded = parse_session(raw).expect("parse");
        assert!(loaded.prefs.lasso);
        assert!(!loaded.prefs.navigation.lasso);
    }

    #[test]
    fn view_key_falls_back_when_graph_id_empty() {
        assert_eq!(view_key(""), "_");
        assert_eq!(view_key("g1"), "g1");
    }
}
