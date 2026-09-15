//! Domain-agnostic hypergraph visualization core.
//!
//! Consumes a [`Hypergraph`] (vertices + hyperedges) and produces a render-agnostic
//! [`HypergraphScene`] plus a headless 3D force layout. No application-specific
//! types (facts, conversations, memory engines) leak into this crate.

pub mod error;
pub mod hull;
pub mod io;
pub mod layout;
pub mod motion;
pub mod project;
pub mod scene;
pub mod schema;
pub mod semantics;

pub use error::VizError;
pub use hull::{HullMesh, hull_from_points};
pub use io::{from_json_str, load_json, save_json};
pub use layout::{ForceLayout3D, LayoutConfig, Vec3, layout_edges_from_scene};
pub use motion::{
    ONE_SHOT_CAP, ONE_SHOT_SECS, StatusMotion, apply_motion_rgba, hyperedge_status_key, id_phase,
    looping_status, one_shot_ids, one_shot_motion, scene_status_snapshot, status_motion,
    status_motion_for, vertex_status_key,
};
pub use project::{Projection, project};
pub use scene::{
    HypergraphScene, LinkKind, NodeRole, SceneHyperedge, SceneId, SceneIndex, SceneLink, SceneMeta,
    SceneNode,
};
pub use schema::{GRAPH_VERSION, GraphMeta, Hyperedge, Hypergraph, Vertex, sample_coauthorship};
pub use semantics::{
    EdgeStatus, Emphasis, HullVisualStyle, LinkVisualStyle, NodeVisualStyle, Rgba,
    emphasis_radius_scale, emphasize, hub_color, hull_style, hull_style_emphasized, hull_style_for,
    hyperedge_color, kind_color, link_color, link_color_for, link_style, link_style_for,
    node_style, parse_status, scaled_radius, status_opacity,
};
