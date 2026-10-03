//! Domain-agnostic hypergraph visualization core.
//!
//! Consumes a [`Hypergraph`] (vertices + hyperedges) and produces a render-agnostic
//! [`HypergraphScene`] plus a headless 3D force layout. No application-specific
//! types (facts, conversations, memory engines, or claim pipelines) leak into
//! this crate — hosts own those concerns.
//!
//! This is the crate other Rust projects should depend on. The native Bevy window
//! lives in `hyper-viz-bevy` and is optional.
//!
//! ```
//! use hyper_viz::prelude::*;
//!
//! let graph = Hypergraph::new()
//!     .with_title("Reactions")
//!     .vertex("h2", "H2", "molecule")
//!     .vertex("o2", "O2", "molecule")
//!     .vertex("h2o", "H2O", "molecule")
//!     .hyperedge("combust", ["h2", "o2", "h2o"], "2H2 + O2 → 2H2O");
//!
//! let scene = graph.project(Projection::Bipartite);
//! assert_eq!(scene.vertices_count(), 3);
//! assert_eq!(scene.hyperedge_count(), 1);
//!
//! let mut layout = ForceLayout3D::from_scene(&scene, LayoutConfig::default());
//! layout.step();
//! assert_eq!(layout.positions.len(), scene.node_count());
//! ```

pub mod error;
pub mod hif;
pub mod hull;
pub mod io;
pub mod layout;
pub mod motion;
pub mod project;
pub mod query;
pub mod scene;
pub mod schema;
pub mod semantics;
pub mod session;

pub use error::VizError;
pub use hif::{
    HifDocument, HifEdge, HifError, HifErrorKind, HifId, HifIncidence, HifNode, load_hif,
    parse_hif, save_hif, serialize_hif,
};
pub use hull::{HullMesh, MAX_HULL_VERTICES, hull_from_points, select_hull_vertices};
pub use io::{
    InputFormat, from_json_str, from_json_str_with_format, load_json, load_json_with_format,
    save_json,
};
pub use layout::{ForceLayout3D, LayoutConfig, Vec3, layout_edges_from_scene};
pub use motion::{
    ONE_SHOT_CAP, ONE_SHOT_SECS, StatusMotion, apply_motion_rgba, hyperedge_status_key, id_phase,
    looping_status, one_shot_ids, one_shot_motion, scene_status_snapshot, status_motion,
    status_motion_for, vertex_status_key,
};
pub use project::{Projection, project};
pub use query::{SceneHits, ScoredHit, scene_hits};
pub use scene::{
    HypergraphScene, LinkKind, NodeRole, SceneHyperedge, SceneId, SceneIndex, SceneLink, SceneMeta,
    SceneNode, neighborhood, scenes_equivalent,
};
pub use schema::{GRAPH_VERSION, GraphMeta, Hyperedge, Hypergraph, Vertex, sample_coauthorship};
pub use semantics::{
    EdgeStatus, Emphasis, HullVisualStyle, LinkVisualStyle, NodeVisualStyle, Rgba, attention_dim,
    emphasis_radius_scale, emphasize, hub_color, hull_style, hull_style_emphasized, hull_style_for,
    hyperedge_color, is_live_status, kind_color, link_color, link_color_for, link_style,
    link_style_for, node_style, parse_status, scaled_radius, status_opacity,
};
pub use session::{
    NavigationPrefs, SESSION_VERSION, ViewerPrefs, ViewerSession, ViewerView, parse_session,
    session_to_json, view_key,
};

/// Common types for host applications that build, project, and lay out a hypergraph.
pub mod prelude {
    pub use crate::{
        ForceLayout3D, GRAPH_VERSION, Hyperedge, Hypergraph, HypergraphScene, LayoutConfig,
        Projection, Vertex, VizError, from_json_str, load_json, project, save_json,
    };
}

#[cfg(test)]
mod public_api {
    use super::prelude::*;

    #[test]
    fn host_can_build_project_and_step_layout() {
        let graph = Hypergraph::new()
            .with_id("g1")
            .vertex("a", "A", "person")
            .vertex("b", "B", "person")
            .hyperedge("e", ["a", "b"], "pair");

        let scene = graph.project(Projection::Bipartite);
        assert_eq!(scene.vertices_count(), 2);
        assert_eq!(scene.hyperedge_count(), 1);
        assert!(scene.warnings.is_empty());

        let mut layout = ForceLayout3D::from_scene(&scene, LayoutConfig::default());
        layout.step();
        assert_eq!(layout.positions.len(), scene.node_count());
        assert!(layout.iterations >= 1);
    }

    #[test]
    fn json_round_trip_is_the_interchange_format() {
        let graph = Hypergraph::new()
            .with_title("demo")
            .vertex("n", "N", "node")
            .hyperedge("loop", ["n"], "unary");
        let raw = serde_json::to_string(&graph).unwrap();
        let loaded = from_json_str(&raw).unwrap();
        assert_eq!(loaded, graph);
        assert_eq!(loaded.version, GRAPH_VERSION);
    }
}
