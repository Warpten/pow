use std::io;
use prost_reflect_build::Builder;
use prost_build::Config;

mod protos;

fn main() -> io::Result<()> {
    Config::default()
        .compile_well_known_types()
        .out_dir("src/proto")
        .include_file("proto.rs")
        .compile_protos(&protos::PROTOS, &["protos"])
        .expect("An error occured while generating types.");

    Builder::new()
        .descriptor_pool("crate::api::DESCRIPTOR_POOL")
        .compile_protos(&["protos/bgs/low/pb/client/account_service.proto"], &["protos"])
}