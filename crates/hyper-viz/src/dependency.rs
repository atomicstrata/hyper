//! Directed import queries, independent of hyperedge membership.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use serde::{Deserialize, Serialize};
use crate::{HypergraphScene, NodeRole};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraversalDepth { Off, Hops(u8), Transitive }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScopeOptions {
    pub imports: TraversalDepth,
    pub dependents: TraversalDepth,
    pub budget: usize,
}
impl Default for ScopeOptions {
    fn default() -> Self { Self { imports: TraversalDepth::Hops(1), dependents: TraversalDepth::Hops(1), budget: 200 } }
}
impl ScopeOptions {
    pub fn normalize(&mut self) {
        self.budget = self.budget.clamp(1, 1000);
        for depth in [&mut self.imports, &mut self.dependents] {
            if let TraversalDepth::Hops(hops) = depth { *hops = (*hops).clamp(1, 6); }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DependencyScope {
    pub center: Option<usize>,
    pub nodes: Vec<usize>,
    pub edges: Vec<(usize, usize)>,
    /// Depths cover all reached nodes, including budget-omitted nodes.
    pub import_depths: HashMap<usize, usize>,
    pub dependent_depths: HashMap<usize, usize>,
    pub total_reachable: usize,
    pub omitted: usize,
}
#[derive(Debug, Clone)]
pub struct DirectedImport {
    pub source: usize,
    pub target: usize,
    pub hyperedge_indices: Vec<usize>,
}
#[derive(Debug, Clone, Default)]
pub struct DependencyIndex {
    ids: HashMap<String, usize>,
    node_ids: Vec<String>,
    outgoing: Vec<Vec<usize>>,
    incoming: Vec<Vec<usize>>,
    relations: Vec<DirectedImport>,
    warnings: Vec<String>,
}

/// Validate direction explicitly; set order and labels never imply direction.
fn endpoints(scene: &HypergraphScene, edge: usize, ids: &HashMap<String, usize>) -> Result<Option<(usize, usize)>, String> {
    let he = &scene.hyperedges[edge];
    if he.kind != "import" { return Ok(None); }
    let direction = (|| {
        if he.member_indices.len() != 2 { return None; }
        let source = *ids.get(he.attrs.get("source")?.as_str()?)?;
        let target = *ids.get(he.attrs.get("target")?.as_str()?)?;
        if !he.member_indices.contains(&source) || !he.member_indices.contains(&target) { return None; }
        Some((source, target))
    })();
    direction.map(Some).ok_or_else(|| format!("Import {} has invalid source/target attributes or members; displayed as an unordered hyperedge", he.id))
}

pub(crate) fn import_warnings(scene: &HypergraphScene) -> Vec<String> {
    let ids = vertex_ids(scene);
    (0..scene.hyperedges.len()).filter_map(|i| endpoints(scene, i, &ids).err()).collect()
}
fn vertex_ids(scene: &HypergraphScene) -> HashMap<String, usize> {
    scene.nodes.iter().enumerate().filter(|(_, n)| n.role == NodeRole::Vertex)
        .map(|(i,n)| (n.id.clone(),i)).collect()
}

impl DependencyIndex {
    pub fn new(scene: &HypergraphScene) -> Self {
        let ids = vertex_ids(scene);
        let mut pairs: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
        let mut warnings = Vec::new();
        for i in 0..scene.hyperedges.len() {
            match endpoints(scene, i, &ids) {
                Ok(Some(pair)) => pairs.entry(pair).or_default().push(i),
                Err(warning) => warnings.push(warning),
                _ => (),
            }
        }
        let node_ids: Vec<_> = scene.nodes.iter().map(|n|n.id.clone()).collect();
        let mut outgoing = vec![Vec::new(); node_ids.len()];
        let mut incoming = outgoing.clone();
        let mut relations: Vec<_> = pairs.into_iter().map(|((source,target),hyperedge_indices)| {
            outgoing[source].push(target); incoming[target].push(source);
            DirectedImport {source,target,hyperedge_indices}
        }).collect();
        for neighbors in outgoing.iter_mut().chain(incoming.iter_mut()) {
            neighbors.sort_by(|a,b|node_ids[*a].cmp(&node_ids[*b]));
        }
        relations.sort_by(|a,b| (&node_ids[a.source],&node_ids[a.target]).cmp(&(&node_ids[b.source],&node_ids[b.target])));
        Self {ids,node_ids,outgoing,incoming,relations,warnings}
    }
    pub fn node_index(&self, id: &str) -> Option<usize> { self.ids.get(id).copied() }
    pub fn id(&self, node: usize) -> Option<&str> { self.node_ids.get(node).map(String::as_str) }
    pub fn imports(&self, node: usize) -> &[usize] { self.outgoing.get(node).map(Vec::as_slice).unwrap_or(&[]) }
    pub fn dependents(&self, node: usize) -> &[usize] { self.incoming.get(node).map(Vec::as_slice).unwrap_or(&[]) }
    pub fn relations(&self) -> &[DirectedImport] { &self.relations }
    pub fn warnings(&self) -> &[String] { &self.warnings }
    pub fn is_empty(&self) -> bool { self.relations.is_empty() }
    fn valid_node(&self, node: usize) -> bool { self.id(node).and_then(|id|self.node_index(id)) == Some(node) }

    pub fn scope(&self, center: usize, options: &ScopeOptions, allowed: &[bool]) -> DependencyScope {
        if !self.valid_node(center) { return DependencyScope::default(); }
        let imports = traverse(&self.outgoing, center, options.imports, allowed);
        let dependents = traverse(&self.incoming, center, options.dependents, allowed);
        self.finish_scope(center, imports, dependents, options.budget)
    }
    pub fn expand(&self, center: usize, previous: &DependencyScope, node: usize, options: &ScopeOptions, allowed: &[bool]) -> DependencyScope {
        if !self.valid_node(center) || !self.valid_node(node) { return previous.clone(); }
        if node != center && !allowed.get(node).copied().unwrap_or(false) { return previous.clone(); }
        let offset = previous.import_depths.get(&node).into_iter().chain(previous.dependent_depths.get(&node)).copied().min().unwrap_or(0);
        let mut imports = previous.import_depths.clone();
        let mut dependents = previous.dependent_depths.clone();
        for (adjacency, depth, into) in [(&self.outgoing, options.imports, &mut imports), (&self.incoming, options.dependents, &mut dependents)] {
            if depth == TraversalDepth::Off { continue; }
            for (next, hops) in traverse(adjacency, node, TraversalDepth::Hops(1), allowed) {
                let value = offset + hops;
                into.entry(next).and_modify(|d|*d=(*d).min(value)).or_insert(value);
            }
        }
        self.finish_scope(center, imports, dependents, options.budget)
    }
    fn finish_scope(&self, center: usize, imports: HashMap<usize, usize>, dependents: HashMap<usize, usize>, budget: usize) -> DependencyScope {
        let mut nodes: Vec<_> = imports.keys().chain(dependents.keys()).copied().collect::<HashSet<_>>().into_iter().collect();
        nodes.sort_by(|a,b| {
            let distance = |node| imports.get(&node).into_iter().chain(dependents.get(&node)).copied().min().unwrap_or(usize::MAX);
            distance(*a).cmp(&distance(*b)).then_with(||self.node_ids[*a].cmp(&self.node_ids[*b]))
        });
        // Center is always first, even if the caller deliberately disabled it.
        if let Some(i) = nodes.iter().position(|i|*i==center) { nodes.remove(i); }
        nodes.insert(0,center);
        let total_reachable = nodes.len();
        nodes.truncate(budget.clamp(1,1000));
        let visible: HashSet<_> = nodes.iter().copied().collect();
        let edges = self.relations.iter().filter(|e|visible.contains(&e.source) && visible.contains(&e.target)).map(|e|(e.source,e.target)).collect();
        DependencyScope {center:Some(center),omitted:total_reachable-nodes.len(),nodes,edges,import_depths:imports,dependent_depths:dependents,total_reachable}
    }
    pub fn shortest_path(&self, source: usize, target: usize, allowed: &[bool]) -> Option<Vec<usize>> {
        if !self.valid_node(source) || !self.valid_node(target) || !allowed.get(source).copied().unwrap_or(false) || !allowed.get(target).copied().unwrap_or(false) { return None; }
        let mut previous = vec![None;self.node_ids.len()];
        previous[source]=Some(source);
        let mut queue=VecDeque::from([source]);
        while let Some(node)=queue.pop_front() {
            if node==target {
                let mut path=vec![node]; let mut current=node;
                while current!=source { current=previous[current]?; path.push(current); }
                path.reverse(); return Some(path);
            }
            for &next in self.imports(node) {
                if allowed.get(next).copied().unwrap_or(false) && previous[next].is_none() {
                    previous[next]=Some(node);queue.push_back(next);
                }
            }
        }
        None
    }
}
fn traverse(adjacency: &[Vec<usize>], source: usize, limit: TraversalDepth, allowed: &[bool]) -> HashMap<usize,usize> {
    let depth_limit = match limit {TraversalDepth::Off=>Some(0),TraversalDepth::Hops(n)=>Some(n as usize),TraversalDepth::Transitive=>None};
    let mut depths=HashMap::from([(source,0)]); let mut queue=VecDeque::from([source]);
    while let Some(node)=queue.pop_front() {
        let depth=depths[&node]; if depth_limit.is_some_and(|limit|depth>=limit) {continue;}
        for &next in &adjacency[node] {
            if allowed.get(next).copied().unwrap_or(false) && !depths.contains_key(&next) {
                depths.insert(next,depth+1);queue.push_back(next);
            }
        }
    }
    depths
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Hypergraph, Hyperedge, Projection};
    fn fixture() -> crate::HypergraphScene {
        let mut g = Hypergraph::new();
        for id in ["a", "b", "c", "d", "x", "z"] { g = g.vertex(id, id, "module"); }
        for (source,target) in [("a","b"),("b","c"),("d","a"),("x","d")] {
            let mut e = Hyperedge::new(format!("{source}:{target}"), [source,target]).with_kind("import");
            e.attrs.insert("source".into(), source.into());
            e.attrs.insert("target".into(), target.into());
            g.add_hyperedge(e);
        }
        g = g.hyperedge("umbrella", ["a","b","c","d","x","z"], "all");
        g.project(Projection::StarCentroid)
    }
    fn ids(scene: &crate::HypergraphScene, nodes: &[usize]) -> Vec<String> {
        let mut ids: Vec<_> = nodes.iter().map(|i|scene.nodes[*i].id.clone()).collect(); ids.sort(); ids
    }
    #[test]
    fn directed_scope_ignores_umbrella_and_expands_directions_independently() {
        let scene=fixture(); let index=DependencyIndex::new(&scene);
        let mut options=ScopeOptions::default(); let allowed=vec![true;scene.nodes.len()];
        assert_eq!(ids(&scene,&index.scope(0,&options,&allowed).nodes), ["a","b","d"]);
        options.imports=TraversalDepth::Hops(2);
        assert_eq!(ids(&scene,&index.scope(0,&options,&allowed).nodes), ["a","b","c","d"]);
        options.imports=TraversalDepth::Hops(1); options.dependents=TraversalDepth::Hops(2);
        assert_eq!(ids(&scene,&index.scope(0,&options,&allowed).nodes), ["a","b","d","x"]);
    }
    #[test]
    fn paths_obey_direction_filters_and_zero_hops() {
        let scene=fixture(); let index=DependencyIndex::new(&scene); let mut allowed=vec![true;6];
        assert_eq!(index.shortest_path(0,2,&allowed), Some(vec![0,1,2]));
        assert_eq!(index.shortest_path(2,0,&allowed), None);
        assert_eq!(index.shortest_path(0,0,&allowed), Some(vec![0]));
        allowed[1]=false; assert_eq!(index.shortest_path(0,2,&allowed),None);
        assert_eq!(index.scope(0,&ScopeOptions {imports:TraversalDepth::Transitive, dependents:TraversalDepth::Off, budget:200},&allowed).nodes,[0]);
    }
    #[test]
    fn cycles_duplicates_and_self_loops_terminate_and_keep_edge_identity() {
        let mut scene=fixture();
        scene.hyperedges.push(scene.hyperedges[0].clone());
        let mut reverse=scene.hyperedges[0].clone(); reverse.id="reverse".into();
        reverse.attrs.insert("source".into(),"b".into()); reverse.attrs.insert("target".into(),"a".into());
        scene.hyperedges.push(reverse);
        let mut self_edge=scene.hyperedges[0].clone(); self_edge.id="self".into(); self_edge.member_indices=vec![0,0];
        self_edge.attrs.insert("target".into(),"a".into()); scene.hyperedges.push(self_edge);
        let index=DependencyIndex::new(&scene);
        let options=ScopeOptions {imports:TraversalDepth::Transitive, dependents:TraversalDepth::Off,budget:200};
        assert_eq!(ids(&scene,&index.scope(0,&options,&[true;6]).nodes),["a","b","c"]);
        assert_eq!(index.relations().iter().find(|e|e.source==0 && e.target==1).unwrap().hyperedge_indices.len(),2);
    }
    #[test]
    fn budgets_are_deterministic_and_expansion_keeps_center_and_counts() {
        let scene=fixture();let index=DependencyIndex::new(&scene);
        let options=ScopeOptions {budget:2,..Default::default()};
        let scope=index.scope(0,&options,&[true;6]);
        assert_eq!(scope.nodes,[0,1]); assert_eq!(scope.total_reachable,3);assert_eq!(scope.omitted,1);
        let expanded=index.expand(0,&scope,1,&ScopeOptions::default(),&[true;6]);
        assert_eq!(ids(&scene,&expanded.nodes),["a","b","c","d"]);
        assert_eq!(index.scope(100,&options,&[true;6]).nodes.len(),0);
        assert_eq!(DependencyIndex::new(&Hypergraph::new().project(Projection::StarCentroid)).scope(0,&options,&[]).nodes.len(),0);
    }
    #[test]
    fn malformed_direction_is_not_guessed_from_label_or_member_order() {
        let mut scene=fixture();scene.hyperedges[0].attrs.insert("target".into(),"z".into());
        scene.hyperedges[1].attrs.remove("source");
        let index=DependencyIndex::new(&scene);assert!(index.imports(0).is_empty());
        assert_eq!(index.warnings().len(),2);
        assert_eq!(scene.hyperedges.len(),5);
    }
}
