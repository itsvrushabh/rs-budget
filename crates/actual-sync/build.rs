use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_dir = PathBuf::from("../../specs/proto");
    let proto_file = proto_dir.join("sync.proto");

    println!("cargo:rerun-if-changed={}", proto_file.display());

    prost_build::Config::new()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(&[proto_file], &[proto_dir])?;

    Ok(())
}
