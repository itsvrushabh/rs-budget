use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_dir = PathBuf::from("../../specs/proto");
    let proto_file = proto_dir.join("sync.proto");

    println!("cargo:rerun-if-changed={}", proto_file.display());

    // Only attempt regeneration if explicitly asked via REGENERATE_PROTO=1
    // The generated Rust code is committed in src/proto/sync.rs to allow builds
    // without requiring an external protoc compiler.
    if std::env::var("REGENERATE_PROTO").is_ok_and(|v| v == "1") {
        prost_build::Config::new()
            .out_dir(PathBuf::from("src/proto"))
            .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
            .compile_protos(&[proto_file], &[proto_dir])?;
    }

    Ok(())
}
