//! Commands operate on the same resources as the native viewer.
use bevy::prelude::*;
use hyper_viz::{Hypergraph, NodeRole, Projection, from_json_str};
use hyper_viz_bevy::host::*;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Resource)]
pub(crate) struct SourceGraph(pub Hypergraph);

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Command {
    LoadJson { json: String },
    SetProjection { projection: ProjectionName },
    Select { selection: Selection },
    ClearSelection,
    Frame,
    SetSettings { settings: Settings },
    Dispose,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ProjectionName {
    Bipartite,
    Clique,
    Star,
}
impl ProjectionName {
    fn projection(self) -> Projection {
        match self {
            Self::Bipartite => Projection::Bipartite,
            Self::Clique => Projection::CliqueExpansion,
            Self::Star => Projection::StarCentroid,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Selection {
    pub vertices: Vec<String>,
    pub hyperedges: Vec<String>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Settings {
    layout_running: Option<bool>,
    iterations_per_frame: Option<usize>,
    focus: Option<bool>,
    attention: Option<bool>,
    lasso: Option<bool>,
    hulls: Option<bool>,
    hide_hubs: Option<bool>,
    wireframe: Option<bool>,
    opacity: Option<f32>,
    vertex_labels: Option<bool>,
    hyperedge_labels: Option<bool>,
    label_mode: Option<LabelMode>,
    label_scale: Option<f32>,
    label_variation: Option<f32>,
    dt: Option<f32>,
    damping: Option<f32>,
    repulsion: Option<f32>,
    attraction: Option<f32>,
    gravity: Option<f32>,
    ideal_length: Option<f32>,
    theta: Option<f32>,
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum LabelMode {
    Capped,
    Selection,
    All,
}

impl Settings {
    fn validate(&self) -> Result<(), String> {
        for (name, value, min, max) in [
            ("opacity", self.opacity, 0., 1.),
            ("labelScale", self.label_scale, 0.1, 10.),
            ("labelVariation", self.label_variation, 0., 1.),
            ("dt", self.dt, 0.001, 1.),
            ("damping", self.damping, 0., 1.),
            ("repulsion", self.repulsion, 0., 1e6),
            ("attraction", self.attraction, 0., 100.),
            ("gravity", self.gravity, 0., 100.),
            ("idealLength", self.ideal_length, 0.01, 1e5),
            ("theta", self.theta, 0.01, 2.),
        ] {
            if value.is_some_and(|v| !v.is_finite() || v < min || v > max) {
                return Err(format!("{name} must be between {min} and {max}"));
            }
        }
        if self
            .iterations_per_frame
            .is_some_and(|v| !(1..=16).contains(&v))
        {
            return Err("iterationsPerFrame must be between 1 and 16".into());
        }
        Ok(())
    }
}

pub(crate) fn apply(world: &mut World, command: Command) -> Result<(), String> {
    match command {
        Command::LoadJson { json } => {
            let graph = from_json_str(&json).map_err(|e| e.to_string())?;
            let projection = world.resource::<LayoutSettings>().projection;
            replace(world, &graph, projection);
            world.resource_mut::<SourceGraph>().0 = graph;
            restore_session_on_first_graph(world);
        }
        Command::SetProjection { projection } => {
            let projection = projection.projection();
            let graph = world.resource::<SourceGraph>().0.clone();
            replace(world, &graph, projection);
        }
        Command::Select { selection } => {
            let scene = &world.resource::<GraphLayout>().scene;
            let nodes = selection
                .vertices
                .iter()
                .map(|id| {
                    scene
                        .nodes
                        .iter()
                        .find(|n| n.id == *id && n.role == NodeRole::Vertex)
                        .map(|n| n.index)
                        .ok_or_else(|| format!("Unknown vertex: {id}"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let edges = selection
                .hyperedges
                .iter()
                .map(|id| {
                    scene
                        .hyperedges
                        .iter()
                        .position(|e| e.id == *id)
                        .ok_or_else(|| format!("Unknown hyperedge: {id}"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            world.resource_scope(|world, mut selected: Mut<SelectionState>| {
                selected.set_hits(nodes, edges);
                selected.remember_ids(&world.resource::<GraphLayout>().scene);
            });
        }
        Command::ClearSelection => world.resource_mut::<SelectionState>().clear(),
        Command::Frame => world.resource_mut::<FrameRequest>().pending = true,
        Command::SetSettings { settings } => apply_settings(world, settings)?,
        Command::Dispose => {
            world.write_message(AppExit::Success);
        }
    }
    Ok(())
}

fn replace(world: &mut World, graph: &Hypergraph, projection: Projection) {
    let mut settings = world.resource::<LayoutSettings>().clone();
    settings.projection = projection;
    let scene = graph.project(projection);
    world.resource_scope(|world, mut selected: Mut<SelectionState>| {
        selected.remember_ids(&world.resource::<GraphLayout>().scene);
        selected.remap_ids(&scene);
    });
    let layout =
        GraphLayout::from_scene_preserve(world.resource::<GraphLayout>(), scene, &settings);
    world.insert_resource(layout);
    world.insert_resource(settings);
    world.resource_mut::<GraphSceneEpoch>().0 += 1;
}

fn apply_settings(world: &mut World, s: Settings) -> Result<(), String> {
    s.validate()?;
    if s.focus == Some(true) && !world.resource::<SelectionState>().has_selection() {
        return Err("Select a vertex or hyperedge before enabling focus".into());
    }
    macro_rules! put {
        ($value:expr, $target:expr) => {
            if let Some(value) = $value {
                $target = value;
            }
        };
    }
    {
        let mut layout = world.resource_mut::<GraphLayout>();
        put!(s.layout_running, layout.running);
        put!(s.iterations_per_frame, layout.iterations_per_frame);
    }
    {
        let mut settings = world.resource_mut::<LayoutSettings>();
        put!(s.iterations_per_frame, settings.iterations_per_frame);
        put!(s.dt, settings.config.dt);
        put!(s.damping, settings.config.damping);
        put!(s.repulsion, settings.config.repulsion);
        put!(s.attraction, settings.config.attraction);
        put!(s.gravity, settings.config.gravity);
        put!(s.ideal_length, settings.config.ideal_length);
        put!(s.theta, settings.config.theta);
    }
    let config = world.resource::<LayoutSettings>().config.clone();
    world.resource_mut::<GraphLayout>().layout.config = config;
    {
        let mut hulls = world.resource_mut::<HyperedgeHullSettings>();
        put!(s.hulls, hulls.enabled);
        put!(s.hide_hubs, hulls.hide_hubs);
        put!(s.wireframe, hulls.wireframe);
        put!(s.opacity, hulls.opacity);
    }
    {
        let mut labels = world.resource_mut::<NodeRenderSettings>();
        put!(s.vertex_labels, labels.labels_enabled);
        put!(s.hyperedge_labels, labels.hyperedge_labels);
        put!(s.label_scale, labels.label_scale);
        put!(s.label_variation, labels.label_variation);
        if let Some(mode) = s.label_mode {
            labels.label_mode = match mode {
                LabelMode::Capped => NodeLabelMode::Capped,
                LabelMode::Selection => NodeLabelMode::SelectionOnly,
                LabelMode::All => NodeLabelMode::All,
            };
        }
    }
    put!(s.attention, world.resource_mut::<AttentionMode>().on);
    put!(s.lasso, world.resource_mut::<LassoState>().enabled);
    if let Some(enabled) = s.focus {
        world.resource_scope(|world, mut focus: Mut<FocusScope>| {
            if enabled != focus.is_active() {
                let selection = world.resource::<SelectionState>();
                toggle_focus(
                    &mut focus,
                    world.resource::<GraphLayout>(),
                    &selection.base_selection,
                    &selection.hyperedges,
                );
            }
        });
    }
    Ok(())
}

pub(crate) fn snapshot(world: &World) -> Value {
    let layout = world.resource::<GraphLayout>();
    let selected = world.resource::<SelectionState>();
    let hulls = world.resource::<HyperedgeHullSettings>();
    let labels = world.resource::<NodeRenderSettings>();
    let settings = world.resource::<LayoutSettings>();
    let c = &settings.config;
    json!({
        "projection": match settings.projection { Projection::Bipartite => "bipartite", Projection::CliqueExpansion => "clique", Projection::StarCentroid => "star" },
        "selection": {
            "vertices": selected.base_selection.iter().filter_map(|&i| layout.scene.nodes.get(i)).filter(|n| n.role == NodeRole::Vertex).map(|n| &n.id).collect::<Vec<_>>(),
            "hyperedges": selected.hyperedges.iter().filter_map(|&i| layout.scene.hyperedges.get(i)).map(|e| &e.id).collect::<Vec<_>>()
        },
        "graph": { "id": layout.scene.meta.id, "title": layout.scene.meta.title,
            "vertices": layout.scene.vertices_count(), "hyperedges": layout.scene.hyperedge_count(),
            "links": layout.link_count, "iterations": layout.iterations(), "warnings": layout.scene.warnings },
        "settings": { "layoutRunning": layout.running, "iterationsPerFrame": layout.iterations_per_frame,
            "focus": world.resource::<FocusScope>().is_active(), "attention": world.resource::<AttentionMode>().on,
            "lasso": world.resource::<LassoState>().enabled, "hulls": hulls.enabled, "hideHubs": hulls.hide_hubs,
            "wireframe": hulls.wireframe, "opacity": hulls.opacity, "vertexLabels": labels.labels_enabled,
            "hyperedgeLabels": labels.hyperedge_labels, "labelMode": match labels.label_mode {
                NodeLabelMode::Capped => "capped", NodeLabelMode::SelectionOnly => "selection", NodeLabelMode::All => "all" },
            "labelScale": labels.label_scale, "labelVariation": labels.label_variation,
            "dt": c.dt, "damping": c.damping, "repulsion": c.repulsion, "attraction": c.attraction,
            "gravity": c.gravity, "idealLength": c.ideal_length, "theta": c.theta },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        let mut world = World::new();
        let graph = hyper_viz::Hypergraph::new()
            .vertex("a", "A", "node")
            .vertex("b", "B", "node")
            .hyperedge("ab", ["a", "b"], "Pair");
        world.insert_resource(GraphLayout::from_scene(
            graph.project(hyper_viz::Projection::Bipartite),
            &LayoutSettings::default(),
        ));
        world.insert_resource(SourceGraph(graph));
        world.init_resource::<LayoutSettings>();
        world.init_resource::<GraphSceneEpoch>();
        world.init_resource::<SelectionState>();
        world.init_resource::<FocusScope>();
        world.init_resource::<FrameRequest>();
        world.init_resource::<AttentionMode>();
        world.init_resource::<LassoState>();
        world.init_resource::<HyperedgeHullSettings>();
        world.init_resource::<NodeRenderSettings>();
        world
    }

    #[test]
    fn invalid_load_retains_graph_and_selection() {
        let mut w = world();
        apply(
            &mut w,
            Command::Select {
                selection: Selection {
                    vertices: vec!["a".into()],
                    hyperedges: vec![],
                },
            },
        )
        .unwrap();
        assert!(
            apply(
                &mut w,
                Command::LoadJson {
                    json: "broken".into()
                }
            )
            .is_err()
        );
        assert_eq!(
            snapshot(&w)["selection"]["vertices"],
            serde_json::json!(["a"])
        );
        assert_eq!(snapshot(&w)["graph"]["vertices"], 2);
    }

    #[test]
    fn queued_scene_change_then_selection_survives_reload_remapping() {
        let mut w = world();
        apply(
            &mut w,
            Command::SetProjection {
                projection: ProjectionName::Clique,
            },
        )
        .unwrap();
        apply(
            &mut w,
            Command::Select {
                selection: Selection {
                    vertices: vec!["b".into()],
                    hyperedges: vec![],
                },
            },
        )
        .unwrap();
        let scene = w.resource::<GraphLayout>().scene.clone();
        w.resource_mut::<SelectionState>().remap_ids(&scene);
        assert_eq!(snapshot(&w)["selection"]["vertices"], json!(["b"]));
    }

    #[test]
    fn settings_are_validated_before_any_change() {
        let mut w = world();
        let settings: Settings =
            serde_json::from_str(r#"{"layoutRunning":false,"opacity":2.0}"#).unwrap();
        assert!(apply(&mut w, Command::SetSettings { settings }).is_err());
        assert!(w.resource::<GraphLayout>().running);
        assert_eq!(w.resource::<HyperedgeHullSettings>().opacity, 0.28);
    }

    #[test]
    fn replacement_and_projection_preserve_stable_selection() {
        let mut w = world();
        apply(
            &mut w,
            Command::Select {
                selection: Selection {
                    vertices: vec!["a".into()],
                    hyperedges: vec![],
                },
            },
        )
        .unwrap();
        apply(
            &mut w,
            Command::SetProjection {
                projection: ProjectionName::Clique,
            },
        )
        .unwrap();
        assert_eq!(
            snapshot(&w)["selection"]["vertices"],
            serde_json::json!(["a"])
        );
        let raw = r#"{"version":"hypergraph.v1","vertices":[{"id":"a","label":"Renamed"}],"hyperedges":[]}"#;
        apply(&mut w, Command::LoadJson { json: raw.into() }).unwrap();
        assert_eq!(
            snapshot(&w)["selection"]["vertices"],
            serde_json::json!(["a"])
        );
        assert_eq!(snapshot(&w)["projection"], "clique");
    }

    #[test]
    fn unknown_selection_rejects_without_clearing_current_selection() {
        let mut w = world();
        apply(
            &mut w,
            Command::Select {
                selection: Selection {
                    vertices: vec!["a".into()],
                    hyperedges: vec![],
                },
            },
        )
        .unwrap();
        assert!(
            apply(
                &mut w,
                Command::Select {
                    selection: Selection {
                        vertices: vec!["missing".into()],
                        hyperedges: vec![]
                    }
                }
            )
            .is_err()
        );
        assert_eq!(
            snapshot(&w)["selection"]["vertices"],
            serde_json::json!(["a"])
        );
    }

    #[test]
    fn hif_preserves_integer_string_large_ids_and_empty_memberships() {
        let mut w = world();
        let raw = r#"{"network-type":"undirected","nodes":[{"node":1},{"node":"1"},{"node":9007199254740993},{"node":"isolated"}],"edges":[{"edge":"empty"}],"incidences":[]}"#;
        apply(&mut w, Command::LoadJson { json: raw.into() }).unwrap();
        let scene = &w.resource::<GraphLayout>().scene;
        assert_eq!(scene.vertices_count(), 4);
        assert_eq!(scene.hyperedge_count(), 1);
        let ids: Vec<_> = scene
            .nodes
            .iter()
            .filter(|n| n.role == NodeRole::Vertex)
            .map(|n| n.id.as_str())
            .collect();
        assert!(ids.contains(&"i:1"));
        assert!(ids.contains(&"s:1"));
        assert!(ids.contains(&"i:9007199254740993"));
        assert!(scene.hyperedges[0].member_indices.is_empty());
    }

    #[test]
    fn incompatible_hif_is_rejected_and_all_projections_work() {
        let mut w = world();
        assert!(
            apply(
                &mut w,
                Command::LoadJson {
                    json: r#"{"network-type":"directed","incidences":[]}"#.into()
                }
            )
            .is_err()
        );
        assert_eq!(snapshot(&w)["graph"]["vertices"], 2);
        for projection in [
            ProjectionName::Star,
            ProjectionName::Clique,
            ProjectionName::Bipartite,
        ] {
            apply(&mut w, Command::SetProjection { projection }).unwrap();
            assert_eq!(snapshot(&w)["graph"]["vertices"], 2);
            assert_eq!(snapshot(&w)["graph"]["hyperedges"], 1);
        }
    }

    #[test]
    fn independent_worlds_do_not_share_graph_or_settings() {
        let mut a = world();
        let b = world();
        apply(
            &mut a,
            Command::LoadJson {
                json: r#"{"version":"hypergraph.v1","vertices":[],"hyperedges":[]}"#.into(),
            },
        )
        .unwrap();
        apply(
            &mut a,
            Command::SetSettings {
                settings: serde_json::from_str(r#"{"layoutRunning":false}"#).unwrap(),
            },
        )
        .unwrap();
        assert_eq!(snapshot(&a)["graph"]["vertices"], 0);
        assert_eq!(snapshot(&b)["graph"]["vertices"], 2);
        assert!(b.resource::<GraphLayout>().running);
    }
}
