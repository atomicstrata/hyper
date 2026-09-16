//! Rank scene nodes and hyperedges with hybrid BM25 + Jaro–Winkler.
//!
//! Exact (and last-token prefix) query terms use Okapi BM25. Leftover tokens
//! align to unused document terms with Jaro–Winkler so a typo still ranks.

use std::collections::{HashMap, HashSet};

use crate::scene::{HypergraphScene, SceneHyperedge, SceneNode};

const K1: f32 = 1.2;
const B: f32 = 0.75;
const JW_THRESHOLD: f32 = 0.88;
const WINKLER_PREFIX_SCALE: f32 = 0.1;
const WINKLER_PREFIX_MAX: usize = 4;

/// A ranked scene hit. `index` is a node index or a hyperedge index depending
/// on which [`SceneHits`] list it lives in.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredHit {
    pub index: usize,
    pub score: f32,
}

/// Ranked localize hits for the current scene. Both lists are score-desc.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneHits {
    pub nodes: Vec<ScoredHit>,
    pub hyperedges: Vec<ScoredHit>,
}

impl SceneHits {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.hyperedges.is_empty()
    }
}

enum DocKind {
    Node(usize),
    Hyperedge(usize),
}

struct Doc {
    kind: DocKind,
    tf: HashMap<String, u32>,
    len: f32,
}

/// Tokenize `query` and rank every scene node and hyperedge.
///
/// Empty or whitespace queries return no hits. A document is kept only when
/// its score is strictly positive.
pub fn scene_hits(scene: &HypergraphScene, query: &str) -> SceneHits {
    let query_tokens = tokenize(query);
    if query_tokens.is_empty() {
        return SceneHits::default();
    }

    let docs = corpus(scene);
    if docs.is_empty() {
        return SceneHits::default();
    }

    let n = docs.len() as f32;
    let mut df: HashMap<&str, u32> = HashMap::new();
    let mut total_len = 0.0f32;
    for doc in &docs {
        total_len += doc.len;
        for term in doc.tf.keys() {
            *df.entry(term.as_str()).or_insert(0) += 1;
        }
    }
    let avgdl = (total_len / n).max(1.0);

    let last = query_tokens.last().cloned();
    let mut seen = HashSet::new();
    let query_terms: Vec<&str> = query_tokens
        .iter()
        .map(String::as_str)
        .filter(|term| seen.insert(*term))
        .collect();

    let mut nodes = Vec::new();
    let mut hyperedges = Vec::new();
    for doc in &docs {
        let score = score_doc(doc, &query_terms, last.as_deref(), n, avgdl, &df);
        if score <= 0.0 {
            continue;
        }
        let hit = ScoredHit {
            index: match doc.kind {
                DocKind::Node(index) | DocKind::Hyperedge(index) => index,
            },
            score,
        };
        match doc.kind {
            DocKind::Node(_) => nodes.push(hit),
            DocKind::Hyperedge(_) => hyperedges.push(hit),
        }
    }
    sort_hits(&mut nodes);
    sort_hits(&mut hyperedges);
    SceneHits { nodes, hyperedges }
}

fn corpus(scene: &HypergraphScene) -> Vec<Doc> {
    let mut docs = Vec::with_capacity(scene.nodes.len() + scene.hyperedges.len());
    for node in &scene.nodes {
        docs.push(doc_from_node(node));
    }
    for (index, he) in scene.hyperedges.iter().enumerate() {
        docs.push(doc_from_hyperedge(index, he));
    }
    docs
}

fn doc_from_node(node: &SceneNode) -> Doc {
    from_fields(
        DocKind::Node(node.index),
        &node.label,
        &node.id,
        &node.kind,
        &node.status,
    )
}

fn doc_from_hyperedge(index: usize, he: &SceneHyperedge) -> Doc {
    from_fields(
        DocKind::Hyperedge(index),
        &he.label,
        &he.id,
        &he.kind,
        &he.status,
    )
}

fn from_fields(kind: DocKind, label: &str, id: &str, kind_field: &str, status: &str) -> Doc {
    let mut tokens = tokenize(label);
    // Label is the display name — emit twice as a cheap field boost.
    tokens.extend(tokenize(label));
    tokens.extend(tokenize(id));
    tokens.extend(tokenize(kind_field));
    tokens.extend(tokenize(status));
    let mut tf = HashMap::new();
    for token in &tokens {
        *tf.entry(token.clone()).or_insert(0) += 1;
    }
    Doc {
        kind,
        len: tokens.len() as f32,
        tf,
    }
}

fn score_doc(
    doc: &Doc,
    query_terms: &[&str],
    last: Option<&str>,
    n: f32,
    avgdl: f32,
    df: &HashMap<&str, u32>,
) -> f32 {
    if doc.tf.is_empty() {
        return 0.0;
    }
    let mut unused: HashSet<&str> = doc.tf.keys().map(String::as_str).collect();
    let mut score = 0.0f32;
    for term in query_terms.iter().copied() {
        if unused.contains(term) {
            score += bm25(term, doc.tf[term], doc.len, n, avgdl, df);
            unused.remove(term);
            continue;
        }
        let try_prefix = last == Some(term);
        if try_prefix && let Some(aligned) = best_prefix(term, &unused, n, df) {
            score += bm25(aligned, doc.tf[aligned], doc.len, n, avgdl, df);
            unused.remove(aligned);
            continue;
        }
        if let Some((aligned, jw)) = best_jaro(term, &unused) {
            score += jw * bm25_tf1(aligned, doc.len, n, avgdl, df);
            unused.remove(aligned);
        }
    }
    score
}

fn best_prefix<'a>(
    query: &str,
    unused: &HashSet<&'a str>,
    n: f32,
    df: &HashMap<&str, u32>,
) -> Option<&'a str> {
    unused
        .iter()
        .copied()
        .filter(|term| term.len() > query.len() && term.starts_with(query))
        .max_by(|a, b| {
            idf(a, n, df)
                .total_cmp(&idf(b, n, df))
                .then_with(|| a.len().cmp(&b.len()))
        })
}

fn best_jaro<'a>(query: &str, unused: &HashSet<&'a str>) -> Option<(&'a str, f32)> {
    unused
        .iter()
        .copied()
        .filter_map(|term| {
            let jw = jaro_winkler(query, term);
            (jw >= JW_THRESHOLD).then_some((term, jw))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

fn bm25(term: &str, tf: u32, dl: f32, n: f32, avgdl: f32, df: &HashMap<&str, u32>) -> f32 {
    let tf = tf as f32;
    let denom = tf + K1 * (1.0 - B + B * (dl / avgdl));
    if denom <= 0.0 {
        return 0.0;
    }
    idf(term, n, df) * (tf * (K1 + 1.0)) / denom
}

fn bm25_tf1(term: &str, dl: f32, n: f32, avgdl: f32, df: &HashMap<&str, u32>) -> f32 {
    bm25(term, 1, dl, n, avgdl, df)
}

fn idf(term: &str, n: f32, df: &HashMap<&str, u32>) -> f32 {
    let df = *df.get(term).unwrap_or(&0) as f32;
    (1.0 + (n - df + 0.5) / (df + 0.5)).ln()
}

fn tokenize(raw: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            current.push(ch.to_ascii_lowercase());
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn jaro_winkler(left: &str, right: &str) -> f32 {
    let jaro = jaro(left, right);
    if jaro <= 0.0 {
        return 0.0;
    }
    let prefix = left
        .chars()
        .zip(right.chars())
        .take(WINKLER_PREFIX_MAX)
        .take_while(|(a, b)| a == b)
        .count() as f32;
    jaro + prefix * WINKLER_PREFIX_SCALE * (1.0 - jaro)
}

fn jaro(left: &str, right: &str) -> f32 {
    if left == right {
        return 1.0;
    }
    let a: Vec<char> = left.chars().collect();
    let b: Vec<char> = right.chars().collect();
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let match_dist = (a.len().max(b.len()) / 2).saturating_sub(1);
    let mut b_matched = vec![false; b.len()];
    let mut a_matches = Vec::new();
    for (i, ch) in a.iter().enumerate() {
        let start = i.saturating_sub(match_dist);
        let end = (i + match_dist + 1).min(b.len());
        if let Some(j) = (start..end).find(|&j| !b_matched[j] && b[j] == *ch) {
            b_matched[j] = true;
            a_matches.push(*ch);
        }
    }
    let m = a_matches.len();
    if m == 0 {
        return 0.0;
    }
    let mut b_matches = Vec::with_capacity(m);
    for (j, matched) in b_matched.iter().enumerate() {
        if *matched {
            b_matches.push(b[j]);
        }
    }
    let transpositions = a_matches
        .iter()
        .zip(b_matches.iter())
        .filter(|(x, y)| x != y)
        .count();
    let m = m as f32;
    let t = transpositions as f32 / 2.0;
    (m / a.len() as f32 + m / b.len() as f32 + (m - t) / m) / 3.0
}

fn sort_hits(hits: &mut [ScoredHit]) {
    hits.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.index.cmp(&b.index))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Projection, project};
    use crate::schema::{Hyperedge, Hypergraph, Vertex};

    fn sample_scene() -> HypergraphScene {
        let mut graph = Hypergraph::new();
        graph.add_vertex(Vertex::new("folder:/ws", "ws").with_kind("folder"));
        graph.add_vertex(Vertex::new("repo:atomicstrata/mind", "mind").with_kind("repo"));
        graph.add_vertex(Vertex::new("wt:/ws/mind", "mind (feat/watch)").with_kind("worktree"));
        graph.add_vertex(
            Vertex::new("pr:atomicstrata/mind#12", "#12 Watch")
                .with_kind("pr")
                .with_status("attention"),
        );
        graph.add_vertex(
            Vertex::new("issue:atomicstrata/mind#3", "#3 Bug")
                .with_kind("issue")
                .with_status("shadowed"),
        );
        graph.add_hyperedge(
            Hyperedge::new(
                "repo-mem:repo:atomicstrata/mind",
                ["repo:atomicstrata/mind", "wt:/ws/mind"],
            )
            .with_label("mind repo")
            .with_kind("repo-membership"),
        );
        graph.add_hyperedge(
            Hyperedge::new(
                "github-backlog:repo:atomicstrata/mind",
                ["repo:atomicstrata/mind", "issue:atomicstrata/mind#3"],
            )
            .with_label("mind GitHub")
            .with_kind("github-backlog")
            .with_status("shadowed"),
        );
        graph.add_hyperedge(
            Hyperedge::new(
                "pr-cluster:pr:atomicstrata/mind#12",
                ["pr:atomicstrata/mind#12", "wt:/ws/mind"],
            )
            .with_label("#12 cluster")
            .with_kind("pr-cluster")
            .with_status("attention"),
        );
        project(&graph, Projection::Bipartite)
    }

    fn node_ids<'a>(scene: &'a HypergraphScene, hits: &'a SceneHits) -> Vec<&'a str> {
        hits.nodes
            .iter()
            .filter_map(|hit| scene.nodes.get(hit.index).map(|n| n.id.as_str()))
            .collect()
    }

    fn he_ids<'a>(scene: &'a HypergraphScene, hits: &'a SceneHits) -> Vec<&'a str> {
        hits.hyperedges
            .iter()
            .filter_map(|hit| scene.hyperedges.get(hit.index).map(|he| he.id.as_str()))
            .collect()
    }

    #[test]
    fn empty_query_is_empty() {
        let scene = sample_scene();
        assert!(scene_hits(&scene, "").is_empty());
        assert!(scene_hits(&scene, "   \t").is_empty());
    }

    #[test]
    fn hash_number_ranks_issue_first() {
        let scene = sample_scene();
        let hits = scene_hits(&scene, "#3");
        let ids = node_ids(&scene, &hits);
        assert_eq!(ids.first().copied(), Some("issue:atomicstrata/mind#3"));
    }

    #[test]
    fn mind_repo_ranks_membership_and_repo_above_issue() {
        let scene = sample_scene();
        let hits = scene_hits(&scene, "mind repo");
        let he = he_ids(&scene, &hits);
        assert_eq!(he.first().copied(), Some("repo-mem:repo:atomicstrata/mind"));

        let ids = node_ids(&scene, &hits);
        let repo = ids.iter().position(|id| *id == "repo:atomicstrata/mind");
        let issue = ids.iter().position(|id| *id == "issue:atomicstrata/mind#3");
        assert!(repo.is_some(), "repo vertex should hit");
        if let (Some(repo), Some(issue)) = (repo, issue) {
            assert!(repo < issue, "repo should outrank issue for 'mind repo'");
        }
    }

    #[test]
    fn feat_watch_and_prefix_hit_worktree() {
        let scene = sample_scene();
        let exact = scene_hits(&scene, "feat/watch");
        assert!(
            node_ids(&scene, &exact).contains(&"wt:/ws/mind"),
            "feat/watch should hit the worktree"
        );
        let prefix = scene_hits(&scene, "wat");
        assert!(
            node_ids(&scene, &prefix).contains(&"wt:/ws/mind"),
            "prefix wat should expand to watch"
        );
    }

    #[test]
    fn typo_feath_watch_still_ranks_worktree() {
        let scene = sample_scene();
        let hits = scene_hits(&scene, "feath/watch");
        let ids = node_ids(&scene, &hits);
        assert!(
            ids.contains(&"wt:/ws/mind"),
            "Jaro–Winkler should align feath → feat; got {ids:?}"
        );
        assert_eq!(ids.first().copied(), Some("wt:/ws/mind"));
    }

    #[test]
    fn common_token_ranks_denser_label_higher() {
        let scene = sample_scene();
        let hits = scene_hits(&scene, "mind");
        let ids = node_ids(&scene, &hits);
        let repo = ids.iter().position(|id| *id == "repo:atomicstrata/mind");
        let issue = ids.iter().position(|id| *id == "issue:atomicstrata/mind#3");
        assert!(repo.is_some() && issue.is_some());
        assert!(
            repo.unwrap() < issue.unwrap(),
            "label-boosted repo should beat issue id mention; order {ids:?}"
        );
    }

    #[test]
    fn tokenize_splits_scheme_ids() {
        assert_eq!(
            tokenize("pr:atomicstrata/mind#12"),
            vec!["pr", "atomicstrata", "mind", "12"]
        );
        assert_eq!(tokenize("feat/watch"), vec!["feat", "watch"]);
        assert_eq!(tokenize("#3"), vec!["3"]);
    }

    #[test]
    fn jaro_winkler_typo_passes_threshold() {
        assert!(jaro_winkler("feath", "feat") >= JW_THRESHOLD);
        assert!(jaro_winkler("feat", "feat") > 0.99);
    }
}
