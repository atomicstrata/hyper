//! Optional Mathlib module categories; directed adjacency stays domain-neutral.
use crate::{HypergraphScene, NodeRole};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Optional Mathlib classifications inferred from source paths, namespaces, or stubs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModuleCategory {
    pub umbrella: bool,
    pub test: bool,
    pub tactic: bool,
    pub external: bool,
}
/// Include/exclude category switches; all categories are included by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModuleFilters {
    pub umbrella: bool,
    pub tests: bool,
    pub tactics: bool,
    pub external: bool,
}
impl Default for ModuleFilters {
    fn default() -> Self {
        Self {
            umbrella: true,
            tests: true,
            tactics: true,
            external: true,
        }
    }
}
impl ModuleFilters {
    /// A vertex is allowed only if every category it belongs to is enabled.
    pub fn allows(&self, category: ModuleCategory) -> bool {
        (self.umbrella || !category.umbrella)
            && (self.tests || !category.test)
            && (self.tactics || !category.tactic)
            && (self.external || !category.external)
    }
}
/// Return categories indexed by scene node. Umbrellas match source directory
/// stems rather than degree; external vertices use `source: "external"` or `stub: true`.
pub fn module_categories(scene: &HypergraphScene) -> Vec<ModuleCategory> {
    let mut directories = HashSet::new();
    for node in &scene.nodes {
        if node.role != NodeRole::Vertex {
            continue;
        }
        if let Some(path) = node.attrs.get("path").and_then(|p| p.as_str()) {
            let mut prefix = path;
            while let Some((parent, _)) = prefix.rsplit_once('/') {
                directories.insert(parent.to_owned());
                prefix = parent;
            }
        }
    }
    scene
        .nodes
        .iter()
        .map(|node| {
            let path = node
                .attrs
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or("");
            ModuleCategory {
                umbrella: path
                    .strip_suffix(".lean")
                    .is_some_and(|stem| directories.contains(stem)),
                test: node.id == "MathlibTest"
                    || node.id.starts_with("MathlibTest.")
                    || node.id == "DownstreamTest"
                    || node.id.starts_with("DownstreamTest."),
                tactic: node.id == "Mathlib.Tactic" || node.id.starts_with("Mathlib.Tactic."),
                external: node.attrs.get("source").and_then(|s| s.as_str()) == Some("external")
                    || node.attrs.get("stub").and_then(|s| s.as_bool()) == Some(true),
            }
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Hypergraph, Projection};
    #[test]
    fn umbrellas_use_source_directories_not_degree() {
        let mut g = Hypergraph::new()
            .vertex("Mathlib", "Mathlib", "module")
            .vertex("Mathlib.Algebra.Basic", "Basic", "module")
            .vertex("busy", "busy", "module")
            .vertex("Aesop", "Aesop", "external");
        for (i, path) in [
            (0, "Mathlib.lean"),
            (1, "Mathlib/Algebra/Basic.lean"),
            (2, "busy.lean"),
        ] {
            g.vertices[i].attrs.insert("path".into(), path.into());
        }
        g.vertices[3].attrs.insert("stub".into(), true.into());
        for n in 0..120 {
            let id = format!("other{n}");
            g = g.vertex(&id, &id, "module");
            let mut edge =
                crate::Hyperedge::new(format!("busy:{id}"), ["busy", &id]).with_kind("import");
            edge.attrs.insert("source".into(), "busy".into());
            edge.attrs.insert("target".into(), id.into());
            g.add_hyperedge(edge);
        }

        let c = module_categories(&g.project(Projection::StarCentroid));
        assert!(c[0].umbrella);
        assert!(!c[2].umbrella);
        assert!(c[3].external);
        assert!(
            !ModuleFilters {
                external: false,
                ..Default::default()
            }
            .allows(c[3])
        );
    }
}
