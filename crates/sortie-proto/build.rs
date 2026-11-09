//! Compiles `proto/` with protox (no `protoc` needed) and generates prost messages for
//! every package plus tonic clients/servers for `sortie.v1` (feature `grpc`).
//! `sortie.link.v1` is compiled only with the feature `link` (the engine side).

use std::path::PathBuf;

use prost::Message as _;

const PROTOS: &[&str] = &[
    "sortie/v1/common.proto",
    "sortie/v1/world.proto",
    "sortie/v1/events.proto",
    "sortie/v1/commands.proto",
    "sortie/v1/state.proto",
    "sortie/v1/logic.proto",
    "sortie/v1/missions.proto",
    "sortie/v1/generator.proto",
    "sortie/v1/engine.proto",
    "sortie/v1/server.proto",
    "sortie/manifest/v1/manifest.proto",
];

const LINK_PROTO: &str = "sortie/link/v1/link.proto";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let proto_root = manifest_dir.join("../../proto").canonicalize()?;
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let mut protos = PROTOS.to_vec();
    if std::env::var_os("CARGO_FEATURE_LINK").is_some() {
        if !proto_root.join(LINK_PROTO).is_file() {
            return Err(format!(
                "the `link` feature needs {LINK_PROTO}, the engine's private link protocol; \
                 it is not part of the Sortie SDK"
            )
            .into());
        }
        protos.push(LINK_PROTO);
    }
    println!("cargo:rerun-if-changed={}", proto_root.display());
    for file in &protos {
        println!("cargo:rerun-if-changed={}", proto_root.join(file).display());
    }

    let fds = protox::compile(&protos, [&proto_root])?;
    std::fs::write(out_dir.join("sortie_descriptor.bin"), fds.encode_to_vec())?;

    let grpc = std::env::var_os("CARGO_FEATURE_GRPC").is_some();
    tonic_prost_build::configure()
        .build_client(grpc)
        .build_server(grpc)
        .build_transport(grpc)
        .generate_default_stubs(grpc)
        .emit_rerun_if_changed(false)
        .compile_fds(fds)?;
    Ok(())
}
