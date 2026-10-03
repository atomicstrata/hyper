//! Offline HIF interchange. A document is not necessarily safe to visualize.
use crate::{Hyperedge, Hypergraph, Vertex};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::OnceLock,
};

/// Location-aware format or viewer compatibility failure (JSON Pointer locations).
#[derive(Debug, Clone, thiserror::Error)]
#[error("HIF {kind:?} at {location}: {reason}")]
pub struct HifError {
    pub kind: HifErrorKind,
    pub location: String,
    pub reason: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HifErrorKind {
    Json,
    Validation,
    Compatibility,
    Io,
}
impl HifError {
    fn new(kind: HifErrorKind, location: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            kind,
            location: location.into(),
            reason: reason.into(),
        }
    }
    fn incompatible(location: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::new(HifErrorKind::Compatibility, location, reason)
    }
}

/// Integer and string IDs inhabit separate namespaces.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(untagged)]
pub enum HifId {
    String(String),
    Integer(Number),
}
impl<'de> Deserialize<'de> for HifId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Value::deserialize(deserializer)? {
            Value::String(s) => Ok(Self::String(s)),
            Value::Number(n) if is_integral(&n) => Ok(Self::Integer(n)),
            _ => Err(serde::de::Error::custom(
                "HIF IDs must be strings or integers",
            )),
        }
    }
}
impl HifId {
    fn viewer_id(&self) -> String {
        match self {
            Self::String(s) => format!("s:{s}"),
            Self::Integer(n) => format!("i:{}", canonical_integer(n)),
        }
    }
    fn label(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Integer(n) => n.to_string(),
        }
    }
}
// A normalized significand and arbitrary-size decimal exponent avoid both
// float rounding and overflow when IDs use JSON's exponent notation.
fn integer_parts(n: &Number) -> Option<(bool, String, num_bigint::BigInt)> {
    let raw = n.to_string();
    let (mantissa, exponent) = raw.split_once(['e', 'E']).unwrap_or((&raw, "0"));
    let exp = exponent.parse::<num_bigint::BigInt>().ok()?;
    let negative = mantissa.starts_with('-');
    let (_, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = mantissa.trim_start_matches('-').replace('.', "");
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return Some((false, "0".into(), 0.into()));
    }
    let trailing = digits.len() - digits.trim_end_matches('0').len();
    let shift = exp - num_bigint::BigInt::from(fraction.len()) + num_bigint::BigInt::from(trailing);
    Some((negative, digits[..digits.len() - trailing].into(), shift))
}
fn is_integral(n: &Number) -> bool {
    integer_parts(n).is_some_and(|(_, _, shift)| shift.sign() != num_bigint::Sign::Minus)
}
fn canonical_integer(n: &Number) -> String {
    let (negative, mut digits, shift) = integer_parts(n).expect("validated integer ID");
    let sign = if negative { "-" } else { "" };
    // Avoid allocating huge expanded strings while preserving exact identity.
    if &shift + num_bigint::BigInt::from(digits.len()) > num_bigint::BigInt::from(10000) {
        return format!("{sign}{digits}e{shift}");
    }
    let zeros = usize::try_from(shift).expect("validated bounded nonnegative exponent");
    digits.extend(std::iter::repeat_n('0', zeros));
    format!("{sign}{digits}")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HifNode {
    pub node: HifId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Map<String, Value>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HifEdge {
    pub edge: HifId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Map<String, Value>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HifIncidence {
    pub node: HifId,
    pub edge: HifId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Map<String, Value>>,
}
/// Validated HIF document; use the interchange functions rather than raw deserialization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HifDocument {
    #[serde(rename = "network-type", skip_serializing_if = "Option::is_none")]
    network_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<Map<String, Value>>,
    incidences: Vec<HifIncidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nodes: Option<Vec<HifNode>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    edges: Option<Vec<HifEdge>>,
}
fn validator() -> &'static jsonschema::Validator {
    static VALIDATOR: OnceLock<jsonschema::Validator> = OnceLock::new();
    VALIDATOR.get_or_init(|| {
        let schema: Value = serde_json::from_str(include_str!("../schemas/hif.schema.json"))
            .expect("bundled HIF schema");
        jsonschema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .build(&schema)
            .expect("valid bundled HIF schema")
    })
}
pub fn parse_hif(raw: &str) -> Result<HifDocument, HifError> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|e| HifError::new(HifErrorKind::Json, "/", e.to_string()))?;
    // jsonschema 0.33's union type validator recognizes only i64/u64 as
    // integers. Validate a shadow copy with mathematically integral IDs
    // normalized to zero; deserialize the untouched arbitrary-precision input.
    let mut validation_value = value.clone();
    for (collection, keys) in [
        ("nodes", &["node"][..]),
        ("edges", &["edge"][..]),
        ("incidences", &["node", "edge"][..]),
    ] {
        if let Some(records) = validation_value
            .get_mut(collection)
            .and_then(Value::as_array_mut)
        {
            for record in records {
                for key in keys {
                    if record
                        .get(*key)
                        .and_then(Value::as_number)
                        .is_some_and(is_integral)
                    {
                        record[*key] = Value::Number(0.into());
                    }
                }
            }
        }
    }
    if let Some(e) = validator().iter_errors(&validation_value).next() {
        return Err(HifError::new(
            HifErrorKind::Validation,
            format!("/{}", e.instance_path.to_string().trim_start_matches('/')),
            e.to_string(),
        ));
    }
    serde_json::from_str(raw)
        .map_err(|e| HifError::new(HifErrorKind::Validation, "/", e.to_string()))
}
pub fn load_hif(path: impl AsRef<Path>) -> Result<HifDocument, HifError> {
    let path = path.as_ref();
    let raw = std::fs::read_to_string(path)
        .map_err(|e| HifError::new(HifErrorKind::Io, path.display().to_string(), e.to_string()))?;
    parse_hif(&raw)
}
pub fn serialize_hif(doc: &HifDocument) -> Result<String, HifError> {
    let raw = serde_json::to_string_pretty(doc)
        .map_err(|e| HifError::new(HifErrorKind::Json, "/", e.to_string()))?;
    parse_hif(&raw)?;
    Ok(raw)
}
pub fn save_hif(path: impl AsRef<Path>, doc: &HifDocument) -> Result<(), HifError> {
    let raw = serialize_hif(doc)?;
    let path = path.as_ref();
    std::fs::write(path, raw)
        .map_err(|e| HifError::new(HifErrorKind::Io, path.display().to_string(), e.to_string()))
}
fn weight(value: &Option<Number>, location: &str) -> Result<Option<f32>, HifError> {
    value
        .as_ref()
        .map(|n| {
            let w = n.as_f64().unwrap_or(f64::INFINITY) as f32;
            if w.is_finite() {
                Ok(w)
            } else {
                Err(HifError::incompatible(
                    location,
                    "weight overflows viewer f32; original is retained in HIF",
                ))
            }
        })
        .transpose()
}
impl HifDocument {
    pub fn incidences(&self) -> &[HifIncidence] {
        &self.incidences
    }
    pub fn nodes(&self) -> Option<&[HifNode]> {
        self.nodes.as_deref()
    }
    pub fn edges(&self) -> Option<&[HifEdge]> {
        self.edges.as_deref()
    }
    pub fn to_hypergraph(&self) -> Result<Hypergraph, HifError> {
        // Revalidate even documents constructed via serde by Rust callers.
        serialize_hif(self)?;
        if self
            .network_type
            .as_deref()
            .is_some_and(|v| v != "undirected")
        {
            return Err(HifError::incompatible(
                "/network-type",
                "viewer supports only undirected hypergraphs",
            ));
        }
        let mut graph = Hypergraph::new();
        graph.meta.attrs = self.metadata.clone().unwrap_or_default();
        let mut nodes = HashSet::new();
        let mut edges = HashMap::new();
        for (i, n) in self.nodes.iter().flatten().enumerate() {
            let id = n.node.viewer_id();
            if !nodes.insert(id.clone()) {
                return Err(HifError::incompatible(
                    format!("/nodes/{i}/node"),
                    "duplicate node declaration",
                ));
            }
            let mut v = Vertex::new(id, n.node.label());
            v.attrs = n.attrs.clone().unwrap_or_default();
            v.weight = weight(&n.weight, &format!("/nodes/{i}/weight"))?;
            graph.vertices.push(v);
        }
        for (i, e) in self.edges.iter().flatten().enumerate() {
            let id = e.edge.viewer_id();
            if edges.insert(id.clone(), graph.hyperedges.len()).is_some() {
                return Err(HifError::incompatible(
                    format!("/edges/{i}/edge"),
                    "duplicate edge declaration",
                ));
            }
            let mut edge = Hyperedge::new(id, Vec::<String>::new()).with_label(e.edge.label());
            edge.attrs = e.attrs.clone().unwrap_or_default();
            edge.weight = weight(&e.weight, &format!("/edges/{i}/weight"))?;
            graph.hyperedges.push(edge);
        }
        let mut pairs = HashSet::new();
        for (i, incidence) in self.incidences.iter().enumerate() {
            for (field, present) in [
                ("direction", incidence.direction.is_some()),
                ("weight", incidence.weight.is_some()),
                ("attrs", incidence.attrs.is_some()),
            ] {
                if present {
                    return Err(HifError::incompatible(
                        format!("/incidences/{i}/{field}"),
                        format!(
                            "incidence {field} cannot be represented by the viewer; retained in HIF"
                        ),
                    ));
                }
            }
            let node = incidence.node.viewer_id();
            let edge = incidence.edge.viewer_id();
            if !pairs.insert((node.clone(), edge.clone())) {
                return Err(HifError::incompatible(
                    format!("/incidences/{i}"),
                    "duplicate incidence pair",
                ));
            }
            if nodes.insert(node.clone()) {
                graph
                    .vertices
                    .push(Vertex::new(&node, incidence.node.label()));
            }
            let index = *edges.entry(edge.clone()).or_insert_with(|| {
                graph.hyperedges.push(
                    Hyperedge::new(edge, Vec::<String>::new()).with_label(incidence.edge.label()),
                );
                graph.hyperedges.len() - 1
            });
            graph.hyperedges[index].vertices.push(node);
        }
        Ok(graph)
    }
    pub fn from_hypergraph(graph: &Hypergraph) -> Result<Self, HifError> {
        fn attrs(
            base: &Map<String, Value>,
            display: Value,
            location: &str,
        ) -> Result<Option<Map<String, Value>>, HifError> {
            let mut result = base.clone();
            if result.contains_key("_hyper_viz") {
                return Err(HifError::incompatible(
                    location,
                    "reserved _hyper_viz attribute already exists",
                ));
            }
            result.insert("_hyper_viz".into(), display);
            Ok(Some(result))
        }
        fn number(w: Option<f32>, location: &str) -> Result<Option<Number>, HifError> {
            w.map(|w| {
                Number::from_f64(w as f64)
                    .ok_or_else(|| HifError::incompatible(location, "non-finite native weight"))
            })
            .transpose()
        }
        if graph.version != crate::GRAPH_VERSION {
            return Err(HifError::incompatible(
                "/version",
                "unsupported native graph version",
            ));
        }
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut incidences = Vec::new();
        let mut node_ids = HashSet::new();
        let mut edge_ids = HashSet::new();
        for (i, v) in graph.vertices.iter().enumerate() {
            if !node_ids.insert(v.id.clone()) {
                return Err(HifError::incompatible(
                    format!("/vertices/{i}/id"),
                    "duplicate native vertex ID",
                ));
            }
            nodes.push(HifNode {
                node: HifId::String(v.id.clone()),
                weight: number(v.weight, &format!("/vertices/{i}/weight"))?,
                attrs: attrs(
                    &v.attrs,
                    serde_json::json!({"label":v.label,"kind":v.kind,"status":v.status}),
                    &format!("/vertices/{i}/attrs"),
                )?,
            });
        }
        for (i, e) in graph.hyperedges.iter().enumerate() {
            if !edge_ids.insert(e.id.clone()) {
                return Err(HifError::incompatible(
                    format!("/hyperedges/{i}/id"),
                    "duplicate native edge ID",
                ));
            }
            let mut members = HashSet::new();
            for (j, v) in e.vertices.iter().enumerate() {
                if !node_ids.contains(v) || !members.insert(v) {
                    return Err(HifError::incompatible(
                        format!("/hyperedges/{i}/vertices/{j}"),
                        "missing native vertex or duplicate membership",
                    ));
                }
                incidences.push(HifIncidence {
                    node: HifId::String(v.clone()),
                    edge: HifId::String(e.id.clone()),
                    weight: None,
                    direction: None,
                    attrs: None,
                });
            }
            edges.push(HifEdge {
                edge: HifId::String(e.id.clone()),
                weight: number(e.weight, &format!("/hyperedges/{i}/weight"))?,
                attrs: attrs(
                    &e.attrs,
                    serde_json::json!({"label":e.label,"kind":e.kind,"status":e.status}),
                    &format!("/hyperedges/{i}/attrs"),
                )?,
            });
        }
        let doc = Self {
            network_type: Some("undirected".into()),
            metadata: attrs(
                &graph.meta.attrs,
                serde_json::json!({"id":graph.meta.id,"title":graph.meta.title}),
                "/meta/attrs",
            )?,
            nodes: Some(nodes),
            edges: Some(edges),
            incidences,
        };
        serialize_hif(&doc)?;
        Ok(doc)
    }
}
