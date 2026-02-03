use std::io;
use prost_reflect_build::Builder;
use prost_build::Config;

fn main() -> io::Result<()> {
    const PROTOS: &[&str; 8] = &["protos/bgs/low/pb/client/account_service.proto",
        "protos/bgs/low/pb/client/authentication_service.proto",
        "protos/bgs/low/pb/client/challenge_service.proto",
        "protos/bgs/low/pb/client/connection_service.proto",
        "protos/bgs/low/pb/client/friends_service.proto",
        "protos/bgs/low/pb/client/game_utilities_service.proto",
        "protos/bgs/low/pb/client/presence_service.proto",
        "protos/bgs/low/pb/client/resource_service.proto"];

    Config::default()
        .compile_well_known_types()
        .out_dir("src/proto")
        .include_file("proto.rs")
        .compile_protos(PROTOS, &["protos"])
        .expect("An error occured while generating types.");

    Builder::new()
        .descriptor_pool("crate::api::DESCRIPTOR_POOL")
        .compile_protos(&["protos/bgs/low/pb/client/account_service.proto"], &["protos"])
}