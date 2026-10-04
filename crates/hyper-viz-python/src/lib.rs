//! Small Python binding boundary; all interchange logic stays in hyper-viz.
use hyper_viz::{HifError, HifErrorKind, InputFormat};
use pyo3::{
    create_exception,
    exceptions::{PyException, PyOSError},
    prelude::*,
};
use pyo3_stub_gen::{define_stub_info_gatherer, derive::*};

create_exception!(_core, HifValidationError, PyException);
create_exception!(_core, HifCompatibilityError, PyException);

fn error(py: Python<'_>, err: HifError) -> PyErr {
    let result = match err.kind {
        HifErrorKind::Compatibility => HifCompatibilityError::new_err(err.to_string()),
        HifErrorKind::Io => PyOSError::new_err(err.to_string()),
        _ => HifValidationError::new_err(err.to_string()),
    };
    let value = result.value(py);
    let _ = value.setattr("location", err.location);
    let _ = value.setattr("reason", err.reason);
    result
}

#[gen_stub_pyclass]
#[pyclass(module = "hyper_viz._core", name = "HifDocument", frozen)]
pub struct Document {
    document: hyper_viz::HifDocument,
}

#[gen_stub_pymethods]
#[pymethods]
impl Document {
    /// Parse and validate HIF without requiring viewer compatibility.
    #[staticmethod]
    fn from_json(py: Python<'_>, raw: &str) -> PyResult<Self> {
        hyper_viz::parse_hif(raw)
            .map(|document| Self { document })
            .map_err(|e| error(py, e))
    }
    /// Load HIF offline from a UTF-8 JSON file.
    #[staticmethod]
    fn load(py: Python<'_>, path: &str) -> PyResult<Self> {
        hyper_viz::load_hif(path)
            .map(|document| Self { document })
            .map_err(|e| error(py, e))
    }
    /// Export native hypergraph.v1 JSON to a new HIF document.
    #[staticmethod]
    fn from_hypergraph_json(py: Python<'_>, raw: &str) -> PyResult<Self> {
        let graph =
            hyper_viz::from_json_str_with_format(raw, InputFormat::Hypergraph).map_err(|e| {
                error(
                    py,
                    HifError {
                        kind: HifErrorKind::Validation,
                        location: "/".into(),
                        reason: e.to_string(),
                    },
                )
            })?;
        hyper_viz::HifDocument::from_hypergraph(&graph)
            .map(|document| Self { document })
            .map_err(|e| error(py, e))
    }
    /// Serialize all HIF properties, including unsupported viewer semantics.
    fn to_json(&self, py: Python<'_>) -> PyResult<String> {
        hyper_viz::serialize_hif(&self.document).map_err(|e| error(py, e))
    }
    /// Save all HIF properties as UTF-8 JSON.
    fn save(&self, py: Python<'_>, path: &str) -> PyResult<()> {
        hyper_viz::save_hif(path, &self.document).map_err(|e| error(py, e))
    }
    /// Convert to viewer JSON, rejecting unsupported semantics and weight overflow.
    fn to_hypergraph_json(&self, py: Python<'_>) -> PyResult<String> {
        let graph = self.document.to_hypergraph().map_err(|e| error(py, e))?;
        serde_json::to_string(&graph).map_err(|e| HifValidationError::new_err(e.to_string()))
    }
}
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Document>()?;
    m.add(
        "HifValidationError",
        m.py().get_type::<HifValidationError>(),
    )?;
    m.add(
        "HifCompatibilityError",
        m.py().get_type::<HifCompatibilityError>(),
    )?;
    Ok(())
}
define_stub_info_gatherer!(stub_info);
