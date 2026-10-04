use hyper_viz::{
    HifDocument, InputFormat, from_json_str, from_json_str_with_format, parse_hif, serialize_hif,
};
use serde_json::{Value, json};

#[test]
fn faithful_roundtrip_and_viewer_identity() {
    let value = json!({"metadata":{"nested":{"list":[null,true,7]}},"nodes":[{"node":1,"attrs":{"label":"integer"}},{"node":"1","weight":2.5},{"node":"isolated"}],"edges":[{"edge":"empty"}],"incidences":[{"node":1,"edge":4},{"node":"1","edge":4}]});
    let doc = parse_hif(&value.to_string()).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&serialize_hif(&doc).unwrap()).unwrap(),
        value
    );
    let graph = doc.to_hypergraph().unwrap();
    assert_eq!(
        graph
            .vertices
            .iter()
            .map(|v| v.id.as_str())
            .collect::<Vec<_>>(),
        ["i:1", "s:1", "s:isolated"]
    );
    assert_eq!(graph.hyperedges[0].vertices.len(), 0);
    assert_eq!(graph.hyperedges[1].vertices, ["i:1", "s:1"]);
    assert_eq!(graph.vertices[1].weight, Some(2.5));
    assert_eq!(graph.meta.attrs["nested"]["list"][0], Value::Null);
}

#[test]
fn preserves_properties_that_viewer_rejects() {
    for value in [
        json!({"network-type":"directed","incidences":[{"node":"a","edge":"e","direction":"head","weight":0.123456789,"attrs":{"role":null}}]}),
        json!({"network-type":"asc","incidences":[]}),
        json!({"incidences":[{"node":1,"edge":2,"attrs":{}}]}),
        json!({"incidences":[{"node":1,"edge":2,"weight":0}]}),
        json!({"incidences":[{"node":1,"edge":2,"direction":"tail"}]}),
    ] {
        let doc = parse_hif(&value.to_string()).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&serialize_hif(&doc).unwrap()).unwrap(),
            value
        );
        let error = doc.to_hypergraph().unwrap_err();
        assert!(!error.location.is_empty());
    }
}

#[test]
fn schema_errors_and_duplicates_are_actionable() {
    for raw in [
        r#"{}"#,
        r#"{"incidences":[{"node":true,"edge":1}]}"#,
        r#"{"incidences":[],"unexpected":1}"#,
    ] {
        assert!(parse_hif(raw).is_err());
    }
    for value in [
        json!({"incidences":[],"nodes":[{"node":1},{"node":1}]}),
        json!({"incidences":[],"edges":[{"edge":"x"},{"edge":"x"}]}),
        json!({"incidences":[{"node":1,"edge":2},{"node":1,"edge":2}]}),
    ] {
        let doc = parse_hif(&value.to_string()).unwrap();
        assert!(
            doc.to_hypergraph()
                .unwrap_err()
                .reason
                .contains("duplicate")
        );
    }
    let doc = parse_hif(r#"{"incidences":[],"nodes":[{"node":1,"weight":1e100}]}"#).unwrap();
    assert!(doc.to_hypergraph().is_err());
}

#[test]
fn loading_detects_hif_and_rejects_ambiguity() {
    assert_eq!(
        from_json_str(r#"{"incidences":[{"node":"a","edge":"e"}]}"#)
            .unwrap()
            .vertices
            .len(),
        1
    );
    assert!(from_json_str(r#"{"incidences":[],"vertices":[]}"#).is_err());
    assert!(from_json_str(r#"{}"#).is_err());
    assert!(from_json_str_with_format(r#"{"incidences":[]}"#, InputFormat::Hypergraph).is_err());
    assert!(
        from_json_str_with_format(
            r#"{"version":"hypergraph.v1","vertices":[]}"#,
            InputFormat::Hif
        )
        .is_err()
    );
}

#[test]
fn native_export_keeps_attributes_and_weights() {
    let mut graph = hyper_viz::sample_coauthorship();
    graph.vertices[0]
        .attrs
        .insert("evidence".into(), json!([null, 3]));
    graph.vertices[0].weight = Some(1.25);
    let doc = HifDocument::from_hypergraph(&graph).unwrap();
    let value: Value = serde_json::from_str(&serialize_hif(&doc).unwrap()).unwrap();
    assert_eq!(value["nodes"][0]["node"], "alice");
    assert_eq!(value["nodes"][0]["weight"], 1.25);
    assert_eq!(value["nodes"][0]["attrs"]["evidence"], json!([null, 3]));
}

#[test]
fn numerical_id_spellings_share_identity_but_big_ids_stay_exact() {
    let doc = parse_hif(r#"{"nodes":[{"node":1.0},{"node":1e0}],"incidences":[]}"#).unwrap();
    assert!(
        doc.to_hypergraph()
            .unwrap_err()
            .reason
            .contains("duplicate")
    );
    let raw = r#"{"nodes":[{"node":184467440737095516160,"attrs":{"precise":0.12345678901234567890123456789}}],"incidences":[]}"#;
    let doc = parse_hif(raw).unwrap();
    let output = serialize_hif(&doc).unwrap();
    assert!(output.contains("184467440737095516160"));
    assert!(output.contains("0.12345678901234567890123456789"));
    assert_eq!(
        doc.to_hypergraph().unwrap().vertices[0].id,
        "i:184467440737095516160"
    );
}

#[test]
fn explicit_native_format_requires_native_envelope() {
    assert!(from_json_str_with_format("{}", InputFormat::Hypergraph).is_err());
}

#[test]
fn file_roundtrip_and_reserved_export_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hif.json");
    let doc = parse_hif(r#"{"incidences":[],"nodes":[],"metadata":{}}"#).unwrap();
    hyper_viz::save_hif(&path, &doc).unwrap();
    assert_eq!(hyper_viz::load_hif(&path).unwrap(), doc);
    let mut graph = hyper_viz::sample_coauthorship();
    graph.vertices[0]
        .attrs
        .insert("_hyper_viz".into(), json!({"original":true}));
    assert!(HifDocument::from_hypergraph(&graph).is_err());
    graph.vertices[0].attrs.clear();
    graph.vertices[0].weight = Some(f32::NAN);
    assert!(HifDocument::from_hypergraph(&graph).is_err());
    graph.vertices[0].weight = None;
    graph.hyperedges[0].vertices.push("missing".into());
    assert!(HifDocument::from_hypergraph(&graph).is_err());
}

#[test]
fn extreme_integer_exponents_do_not_panic_or_split_identity() {
    let doc = parse_hif(r#"{"nodes":[{"node":10e9223372036854775807}],"incidences":[]}"#).unwrap();
    assert_eq!(
        doc.to_hypergraph().unwrap().vertices[0].id,
        "i:1e9223372036854775808"
    );
    let doc =
        parse_hif(r#"{"nodes":[{"node":0.1e10003},{"node":1e10002}],"incidences":[]}"#).unwrap();
    assert!(
        doc.to_hypergraph()
            .unwrap_err()
            .reason
            .contains("duplicate")
    );
    assert!(
        parse_hif(r#"{"nodes":[{"node":1e-999999999999999999999999}],"incidences":[]}"#).is_err()
    );
}
