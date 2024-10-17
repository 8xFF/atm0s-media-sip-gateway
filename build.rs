#[cfg(feature = "build-protobuf")]
use prost_build::Config;
use std::io::Result;

fn main() -> Result<()> {
    #[cfg(feature = "build-protobuf")]
    Config::new()
        .out_dir("src/protocol/protobuf")
        .include_file("mod.rs")
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&["./protobuf/sip_gateway.proto"], &["./protobuf"])?;
    Ok(())
}
