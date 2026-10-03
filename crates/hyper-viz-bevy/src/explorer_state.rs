use bevy::prelude::*;
use hyper_viz::session::{ExplorerSnapshot, ViewMode};
use hyper_viz::{
    DependencyIndex, DependencyScope, HypergraphScene, ModuleCategory, module_categories,
};

#[derive(Resource, Default)]
pub struct ExplorerState {
    pub mode: ViewMode,
    pub requested: ViewMode,
    pub user_mode: bool,
    pub current: ExplorerSnapshot,
    pub index: DependencyIndex,
    pub categories: Vec<ModuleCategory>,
    pub scope: DependencyScope,
    pub canonical: DependencyScope,
    pub allowed: Vec<bool>,
    pub path: Option<Vec<usize>>,
    pub path_blocked: bool,
    pub epoch: Option<u64>,
    pub history: Vec<ExplorerSnapshot>,
    pub cursor: usize,
    corpus: Vec<(usize, String)>,
    search_cache: std::collections::HashMap<String, Vec<usize>>,
    pub group: Option<usize>,
    pub fit_requested: bool,
}
impl ExplorerState {
    pub fn choose_mode(&mut self, mode: ViewMode) {
        self.mode = mode;
        self.user_mode = true;
    }
    pub fn replace_scene(&mut self, scene: &HypergraphScene, epoch: u64) {
        self.index = DependencyIndex::new(scene);
        self.categories = module_categories(scene);
        self.corpus = scene
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.role == hyper_viz::NodeRole::Vertex)
            .map(|(i, n)| (i, n.id.to_lowercase()))
            .collect();
        self.search_cache.clear();
        self.group = None;
        self.epoch = Some(epoch);
        if !self.user_mode {
            if self.requested == ViewMode::Auto {
                self.mode = if self.index.is_empty() {
                    ViewMode::Spatial
                } else {
                    ViewMode::Dependencies
                };
            } else {
                self.mode = self.requested;
            }
        }
        self.rebuild();
    }
    pub fn select_module(&mut self, id: &str) {
        if self.index.node_index(id).is_none() {
            return;
        }
        if let Some(snapshot) = self.history.get_mut(self.cursor) {
            *snapshot = self.current.clone();
        }
        self.fit_requested = true;
        self.current.center = Some(id.to_owned());
        self.current.expanded_ids.clear();
        self.current.pan = [0., 0.];
        self.current.zoom = 1.;
        self.group = None;
        self.rebuild();
        self.record();
    }
    pub fn search(&mut self, query: &str) -> &[usize] {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return &[];
        }
        if self.search_cache.contains_key(&query) {
            return &self.search_cache[&query];
        }

        let mut candidates: Vec<_> = self
            .corpus
            .iter()
            .filter_map(|(i, id)| {
                let rank = if id == &query {
                    0
                } else if id.starts_with(&query) {
                    1
                } else if id.contains(&query) {
                    2
                } else {
                    let mut letters = query.chars();
                    let mut next = letters.next();
                    for c in id.chars() {
                        if next == Some(c) {
                            next = letters.next();
                        }
                    }
                    if next.is_some() {
                        return None;
                    }
                    3
                };
                Some((rank, id, *i))
            })
            .collect();
        candidates.sort();
        let results = candidates.into_iter().take(20).map(|(_, _, i)| i).collect();
        if self.search_cache.len() >= 4 {
            self.search_cache.clear();
        }
        self.search_cache.insert(query.clone(), results);
        &self.search_cache[&query]
    }
    pub fn snapshot(&self) -> ExplorerSnapshot {
        self.current.clone()
    }
    pub fn restore(&mut self, mut snapshot: ExplorerSnapshot) {
        snapshot.normalize();
        self.fit_requested = false;
        self.current = snapshot;
        self.group = None;
        self.rebuild();
    }
    pub fn record(&mut self) {
        if self.history.get(self.cursor) == Some(&self.current) {
            return;
        }
        if !self.history.is_empty() {
            self.history.truncate(self.cursor + 1);
        }
        self.history.push(self.current.clone());
        if self.history.len() > 32 {
            self.history.remove(0);
        }
        self.cursor = self.history.len() - 1;
    }
    pub fn back(&mut self) {
        if let Some(snapshot) = self.history.get_mut(self.cursor) {
            *snapshot = self.current.clone();
        }
        if self.cursor > 0 {
            self.cursor -= 1;
            self.restore(self.history[self.cursor].clone());
        }
    }
    pub fn forward(&mut self) {
        if let Some(snapshot) = self.history.get_mut(self.cursor) {
            *snapshot = self.current.clone();
        }
        if self.cursor + 1 < self.history.len() {
            self.cursor += 1;
            self.restore(self.history[self.cursor].clone());
        }
    }
    pub fn rebuild(&mut self) {
        self.current.normalize();
        self.allowed = self
            .categories
            .iter()
            .map(|c| self.current.filters.allows(*c))
            .collect();
        self.scope = DependencyScope::default();
        self.canonical = DependencyScope::default();
        self.path = None;
        self.path_blocked = false;
        let Some(center) = self
            .current
            .center
            .as_deref()
            .and_then(|id| self.index.node_index(id))
        else {
            return;
        };
        self.allowed[center] = true;
        self.scope = self
            .index
            .scope(center, &self.current.options, &self.allowed);
        self.canonical = self.index.scope(
            center,
            &self.current.options,
            &vec![true; self.allowed.len()],
        );
        if let Some(target) = self
            .current
            .target
            .as_deref()
            .and_then(|id| self.index.node_index(id))
        {
            self.path = self.index.shortest_path(center, target, &self.allowed);
            self.path_blocked = self.path.is_none()
                && self
                    .index
                    .shortest_path(center, target, &vec![true; self.allowed.len()])
                    .is_some();
        }
        if let Some(path) = &self.path {
            self.scope = self
                .index
                .include_path(&self.scope, path, self.current.options.budget);
            self.canonical =
                self.index
                    .include_path(&self.canonical, path, self.current.options.budget);
        }
        for id in &self.current.expanded_ids {
            if let Some(node) = self.index.node_index(id) {
                self.scope = self.index.expand(
                    center,
                    &self.scope,
                    node,
                    &self.current.options,
                    &self.allowed,
                );
                self.canonical = self.index.expand(
                    center,
                    &self.canonical,
                    node,
                    &self.current.options,
                    &vec![true; self.allowed.len()],
                );
            }
        }
        if let Some(path) = &self.path {
            self.scope = self
                .index
                .include_path(&self.scope, path, self.current.options.budget);
            self.canonical =
                self.index
                    .include_path(&self.canonical, path, self.current.options.budget);
        }
    }
    pub fn expand(&mut self, id: String) {
        if !self.current.expanded_ids.contains(&id) {
            self.current.expanded_ids.push(id);
            self.rebuild();
            self.record();
        }
    }
}
pub fn spatial_mode(state: Option<Res<ExplorerState>>) -> bool {
    state.is_none_or(|s| s.mode == ViewMode::Spatial)
}

pub fn sync_explorer(
    layout: Option<Res<crate::graph::GraphLayout>>,
    epoch: Res<crate::graph::GraphSceneEpoch>,
    mut state: ResMut<ExplorerState>,
    settings: Res<crate::graph::LayoutSettings>,
    store: Option<Res<crate::session::SessionStore>>,
) {
    let Some(layout) = layout else {
        return;
    };
    if state.epoch == Some(epoch.0) {
        return;
    }
    let first = state.epoch.is_none();
    state.replace_scene(&layout.scene, epoch.0);
    if first
        && let Some(store) = store
        && store.enabled
        && let Some(saved) = store
            .session
            .views
            .get(&hyper_viz::view_key(&layout.scene.meta.id))
            .and_then(|v| v.explorer.as_ref())
    {
        if state.requested == ViewMode::Auto && saved.mode != ViewMode::Auto {
            state.choose_mode(saved.mode);
        }
        state.restore(saved.snapshot.clone());
        state.history = saved.history.iter().rev().take(32).cloned().collect();
        state.history.reverse();
        state.cursor = saved.cursor.min(state.history.len().saturating_sub(1));
    }
    if first && let Some(id) = &settings.initial_module {
        state.select_module(id);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use hyper_viz::{Hyperedge, Hypergraph, Projection};
    fn scene() -> HypergraphScene {
        let mut g = Hypergraph::new()
            .vertex("a", "a", "module")
            .vertex("ab", "ab", "module")
            .vertex("c", "c", "module");
        let mut e = Hyperedge::new("a:ab", ["a", "ab"]).with_kind("import");
        e.attrs.insert("source".into(), "a".into());
        e.attrs.insert("target".into(), "ab".into());
        g.add_hyperedge(e);
        g.project(Projection::StarCentroid)
    }
    #[test]
    fn exact_selection_history_and_reload_use_stable_ids() {
        let mut s = ExplorerState::default();
        s.replace_scene(&scene(), 0);
        assert_eq!(s.search("a")[0], 0);
        assert!(s.current.center.is_none());
        s.select_module("a");
        assert_eq!(s.scope.nodes, vec![0, 1]);
        s.current.filters.tests = false;
        s.expand("ab".into());
        s.select_module("c");
        s.back();
        assert_eq!(s.current.center.as_deref(), Some("a"));
        assert!(!s.current.filters.tests);
        assert_eq!(s.current.expanded_ids, ["ab"]);
        s.forward();
        assert_eq!(s.current.center.as_deref(), Some("c"));
        let removed = Hypergraph::new()
            .vertex("a", "a", "module")
            .project(Projection::StarCentroid);
        s.replace_scene(&removed, 1);
        assert_eq!(s.current.center.as_deref(), Some("c"));
        assert!(s.scope.nodes.is_empty());
    }
    #[test]
    fn snapshot_sanitizes_nonfinite_values_and_bounds() {
        let mut snapshot = ExplorerSnapshot {
            pan: [f32::NAN, 2.],
            zoom: f32::INFINITY,
            ..Default::default()
        };
        snapshot.options.budget = 5000;
        let mut state = ExplorerState::default();
        state.restore(snapshot);
        assert_eq!(state.current.pan, [0., 0.]);
        assert_eq!(state.current.zoom, 1.);
        assert_eq!(state.current.options.budget, 1000);
    }
    #[test]
    fn explicit_mode_survives_a_scene_reload() {
        let mut state = ExplorerState::default();
        state.replace_scene(&scene(), 0);
        state.choose_mode(ViewMode::Spatial);
        state.replace_scene(&scene(), 1);
        assert_eq!(state.mode, ViewMode::Spatial);
    }
    #[test]
    fn filtering_an_intermediate_stops_replayed_expansion() {
        let mut graph = Hypergraph::new();
        for id in ["A", "MathlibTest.B", "C", "D"] {
            graph = graph.vertex(id, id, "module");
        }
        for (a, b) in [("A", "MathlibTest.B"), ("MathlibTest.B", "C"), ("C", "D")] {
            let mut edge = Hyperedge::new(format!("{a}:{b}"), [a, b]).with_kind("import");
            edge.attrs.insert("source".into(), a.into());
            edge.attrs.insert("target".into(), b.into());
            graph.add_hyperedge(edge);
        }
        let mut state = ExplorerState::default();
        state.replace_scene(&graph.project(Projection::StarCentroid), 0);
        state.select_module("A");
        state.current.options.imports = hyper_viz::TraversalDepth::Hops(2);
        state.rebuild();
        state.expand("C".into());
        assert_eq!(state.scope.nodes.len(), 4);
        state.current.filters.tests = false;
        state.rebuild();
        assert_eq!(state.scope.nodes, [state.index.node_index("A").unwrap()]);
    }
    #[test]
    fn tracing_a_path_adds_steps_beyond_depth_and_discloses_budget_omissions() {
        let mut graph = Hypergraph::new();
        for id in ["A", "B", "C", "D"] {
            graph = graph.vertex(id, id, "module");
        }
        for (a, b) in [("A", "B"), ("B", "C"), ("C", "D")] {
            let mut edge = Hyperedge::new(format!("{a}:{b}"), [a, b]).with_kind("import");
            edge.attrs.insert("source".into(), a.into());
            edge.attrs.insert("target".into(), b.into());
            graph.add_hyperedge(edge);
        }
        let mut state = ExplorerState::default();
        state.replace_scene(&graph.project(Projection::StarCentroid), 0);
        state.select_module("A");
        state.current.target = Some("D".into());
        state.rebuild();
        assert_eq!(state.scope.nodes.len(), 4);
        assert_eq!(state.path.as_ref().unwrap().len(), 4);
        state.current.options.budget = 2;
        state.rebuild();
        assert_eq!(state.scope.nodes.len(), 2);
        assert_eq!(state.scope.omitted, 2);
        assert_eq!(state.path.as_ref().unwrap().len(), 4);
    }
    #[test]
    fn displayed_path_modules_can_be_expanded() {
        let mut graph = Hypergraph::new();
        for id in ["A", "B", "C", "D"] {
            graph = graph.vertex(id, id, "module");
        }
        for (a, b) in [("A", "B"), ("B", "C"), ("C", "D")] {
            let mut edge = Hyperedge::new(format!("{a}:{b}"), [a, b]).with_kind("import");
            edge.attrs.insert("source".into(), a.into());
            edge.attrs.insert("target".into(), b.into());
            graph.add_hyperedge(edge);
        }
        let mut state = ExplorerState::default();
        state.replace_scene(&graph.project(Projection::StarCentroid), 0);
        state.select_module("A");
        state.current.target = Some("C".into());
        state.rebuild();
        assert_eq!(state.scope.nodes.len(), 3);
        state.expand("C".into());
        assert_eq!(state.scope.nodes.len(), 4);
    }
}
