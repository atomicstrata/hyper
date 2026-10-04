fn main() -> pyo3_stub_gen::Result<()> {
    let mut stubs = hyper_viz_python::stub_info()?;
    // The parent is handwritten Python: a generated parent stub would hide
    // show(), ViewerHandle, and exception aliases from type checkers.
    stubs
        .modules
        .retain(|name, _| name.starts_with("hyper_viz._core"));
    stubs.generate()?;
    let root = stubs.python_root.join("hyper_viz");
    std::fs::rename(root.join("_core/__init__.pyi"), root.join("_core.pyi"))?;
    let stub = root.join("_core.pyi");
    let content = std::fs::read_to_string(&stub)?;
    std::fs::write(stub, format!("{}\n", content.trim_end()))?;
    std::fs::remove_dir(root.join("_core"))?;
    Ok(())
}
