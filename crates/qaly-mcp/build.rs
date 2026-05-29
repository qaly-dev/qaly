fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        // Make the perception types JSON-serializable so the MCP `perceive`
        // tool can return them directly.
        .type_attribute("qaly.v1.Element", "#[derive(serde::Serialize)]")
        .type_attribute("qaly.v1.BBox", "#[derive(serde::Serialize)]")
        .compile_protos(&["../../proto/qaly.proto"], &["../../proto"])?;
    Ok(())
}
