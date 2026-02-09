use std::io;
use prost_reflect_build::Builder;
use prost_build::Config;

fn main() -> io::Result<()> {
    // This crate's build step only requires actual types for all
    // extensions.
    Config::default()
        .compile_well_known_types()
        .out_dir("src/proto")
        .include_file("proto.rs")
        .compile_protos(&[
            "bgs/low/pb/client/global_extensions/field_options.proto",
            "bgs/low/pb/client/global_extensions/message_options.proto",
            "bgs/low/pb/client/global_extensions/method_options.proto",
            "bgs/low/pb/client/global_extensions/range.proto",
            "bgs/low/pb/client/global_extensions/register_method_types.proto",
            "bgs/low/pb/client/global_extensions/routing.proto",
            "bgs/low/pb/client/global_extensions/service_options.proto",
        ], &["protos"])
        .expect("An error occured while generating types.");

    // However we need reflection data for all services.
    Builder::new()
        .descriptor_pool("crate::api::DESCRIPTOR_POOL")
        .compile_protos(&[
            "protos/bgs/low/pb/client/account_service.proto",
            "protos/bgs/low/pb/client/authentication_service.proto",
            "protos/bgs/low/pb/client/challenge_service.proto",
            "protos/bgs/low/pb/client/connection_service.proto",
            "protos/bgs/low/pb/client/friends_service.proto",
            "protos/bgs/low/pb/client/game_utilities_service.proto",
            "protos/bgs/low/pb/client/presence_service.proto",
            "protos/bgs/low/pb/client/resource_service.proto",
            "protos/bgs/low/pb/client/v1/block_list_service.proto",
            "protos/bgs/low/pb/client/v1/club_membership_service.proto",
            "protos/bgs/low/pb/client/v2/notification_service.proto",
            "protos/bgs/low/pb/client/v2/report_service.proto",
            "protos/bgs/low/pb/client/v2/whisper_service.proto",
        ], &["protos"])
}