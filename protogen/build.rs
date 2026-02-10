use prost::Message;
use prost_build::Config;
use prost_reflect_build::Builder;
use prost_types::FileDescriptorSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::io;

fn make_file_descriptor_set<S: AsRef<OsStr>>(protoc: S, fd: impl AsRef<OsStr>, paths: &[impl AsRef<Path>], includes: &[impl AsRef<Path>]) -> Option<FileDescriptorSet> {
    let mut command = Command::new(protoc);
    includes.iter().for_each(|i| {
        command.arg(format!("-I{}", i.as_ref().display()));
    });

    paths.iter().for_each(|p| {
        command.arg(p.as_ref());
    });

    // Also include all dependencies so that the set is self-contained.
    command.arg("--include_imports");
    command.arg("-o").arg(&fd);

    match command.output() {
        Ok(_) => (),
        Err(err) => panic!("An error occurred while generating a descriptor set: {}", err),
    };

    match std::fs::read(fd.as_ref()) {
        Ok(fd) => FileDescriptorSet::decode(fd.as_slice()).ok(),
        Err(err) => panic!("An error occurred while reading the descriptor set '{}': {}", fd.as_ref().display(), err)
    }
}

fn main() -> io::Result<()> {
    let root_directory = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(dir) => std::fs::canonicalize(PathBuf::from(dir).join("..")).unwrap().display().to_string(),
        Err(_) => panic!("CARGO_MANIFEST_DIR is not set"),
    };

    let protoc = format!("{}/protoc", root_directory);
    assert!(matches!(std::fs::exists(&protoc), Ok(true)), "protoc not found in {}", root_directory);

    let proto_root = format!("{}/protos", root_directory);
    assert!(matches!(std::fs::exists(&proto_root), Ok(true)), "/protos/ subdirectory missing in {}", root_directory);

    macro_rules! make_abs {
        ($p:expr) => {
            format!("{}/{}", &proto_root, $p)
        };
    }

    let protos = [
        make_abs!("bgs/low/pb/client/account_service.proto"),
        make_abs!("bgs/low/pb/client/account_types.proto"),
        make_abs!("bgs/low/pb/client/attribute_types.proto"),
        make_abs!("bgs/low/pb/client/authentication_service.proto"),
        make_abs!("bgs/low/pb/client/challenge_service.proto"),
        make_abs!("bgs/low/pb/client/content_handle_types.proto"),
        make_abs!("bgs/low/pb/client/connection_service.proto"),
        make_abs!("bgs/low/pb/client/embed_types.proto"),
        make_abs!("bgs/low/pb/client/entity_types.proto"),
        make_abs!("bgs/low/pb/client/ets_types.proto"),
        make_abs!("bgs/low/pb/client/event_view_types.proto"),
        make_abs!("bgs/low/pb/client/friends_service.proto"),
        make_abs!("bgs/low/pb/client/friends_types.proto"),
        make_abs!("bgs/low/pb/client/game_utilities_service.proto"),
        make_abs!("bgs/low/pb/client/game_utilities_types.proto"),
        make_abs!("bgs/low/pb/client/invitation_types.proto"),
        make_abs!("bgs/low/pb/client/message_types.proto"),
        make_abs!("bgs/low/pb/client/notification_types.proto"),
        make_abs!("bgs/low/pb/client/presence_listener.proto"),
        make_abs!("bgs/low/pb/client/presence_service.proto"),
        make_abs!("bgs/low/pb/client/presence_types.proto"),
        make_abs!("bgs/low/pb/client/profanity_filter_config.proto"),
        make_abs!("bgs/low/pb/client/resource_service.proto"),
        make_abs!("bgs/low/pb/client/role_types.proto"),
        make_abs!("bgs/low/pb/client/rpc_config.proto"),
        make_abs!("bgs/low/pb/client/rpc_types.proto"),
        make_abs!("bgs/low/pb/client/semantic_version.proto"),
        make_abs!("bgs/low/pb/client/voice_types.proto"),

        make_abs!("bgs/low/pb/client/api/client/v1/block_list_listener.proto"),
        make_abs!("bgs/low/pb/client/api/client/v1/block_list_service.proto"),
        make_abs!("bgs/low/pb/client/api/client/v1/block_list_types.proto"),
        make_abs!("bgs/low/pb/client/api/client/v1/channel_id.proto"),
        make_abs!("bgs/low/pb/client/api/client/v1/channel_types.proto"),
        make_abs!("bgs/low/pb/client/api/client/v1/club_membership_service.proto"),
        make_abs!("bgs/low/pb/client/api/client/v1/club_membership_types.proto"),
        make_abs!("bgs/low/pb/client/api/client/v1/club_stream.proto"),
        make_abs!("bgs/low/pb/client/api/client/v1/club_types.proto"),
        make_abs!("bgs/low/pb/client/api/client/v1/club_member.proto"),
        make_abs!("bgs/low/pb/client/api/client/v1/club_invitation.proto"),
        make_abs!("bgs/low/pb/client/api/client/v1/club_enum.proto"),
        make_abs!("bgs/low/pb/client/api/client/v1/club_role.proto"),
        make_abs!("bgs/low/pb/client/api/client/v1/club_range_set.proto"),
        make_abs!("bgs/low/pb/client/api/client/v1/club_core.proto"),
        make_abs!("bgs/low/pb/client/api/client/v1/club_ban.proto"),
        make_abs!("bgs/low/pb/client/api/client/v1/club_name_generator.proto"),

        make_abs!("bgs/low/pb/client/api/client/v2/notification_service.proto"),
        make_abs!("bgs/low/pb/client/api/client/v2/notification_types.proto"),
        make_abs!("bgs/low/pb/client/api/client/v2/report_service.proto"),
        make_abs!("bgs/low/pb/client/api/client/v2/report_types.proto"),
        make_abs!("bgs/low/pb/client/api/client/v2/whisper_listener.proto"),
        make_abs!("bgs/low/pb/client/api/client/v2/whisper_service.proto"),

        make_abs!("bgs/low/pb/client/api/common/v1/club_enum.proto"),
        make_abs!("bgs/low/pb/client/api/common/v1/club_tag.proto"),
        make_abs!("bgs/low/pb/client/api/common/v1/club_type.proto"),
        make_abs!("bgs/low/pb/client/api/common/v1/club_core.proto"),
        make_abs!("bgs/low/pb/client/api/common/v1/club_member_id.proto"),
        make_abs!("bgs/low/pb/client/api/common/v1/embed_types.proto"),
        make_abs!("bgs/low/pb/client/api/common/v1/event_view_types.proto"),
        make_abs!("bgs/low/pb/client/api/common/v1/invitation_types.proto"),
        make_abs!("bgs/low/pb/client/api/common/v1/message_types.proto"),
        make_abs!("bgs/low/pb/client/api/common/v1/voice_types.proto"),

        make_abs!("bgs/low/pb/client/api/common/v2/attribute_types.proto"),
        make_abs!("bgs/low/pb/client/api/common/v2/game_account_handle.proto"),
        make_abs!("bgs/low/pb/client/api/common/v2/whisper_types.proto"),

        make_abs!("bgs/low/pb/client/global_extensions/field_options.proto"),
        make_abs!("bgs/low/pb/client/global_extensions/message_options.proto"),
        make_abs!("bgs/low/pb/client/global_extensions/method_options.proto"),
        make_abs!("bgs/low/pb/client/global_extensions/range.proto"),
        make_abs!("bgs/low/pb/client/global_extensions/register_method_types.proto"),
        make_abs!("bgs/low/pb/client/global_extensions/routing.proto"),
        make_abs!("bgs/low/pb/client/global_extensions/service_options.proto"),

        make_abs!("google/protobuf/descriptor.proto"),
    ];

    let fds = match make_file_descriptor_set(&protoc,
        format!("{}/fd.bin", root_directory),
        &protos,
        &[&proto_root])
    {
        Some(fds) => fds,
        _ => panic!("Failed to create a file descriptor set for extensions")
    };

    let mut config = Config::default();
    config.protoc_executable(protoc)
        .compile_well_known_types()
        .out_dir("src/proto")
        .include_file("proto.rs");

    config.compile_fds(fds).expect("Failed to compile file descriptor sets");

    // However we need reflection data for all services.
    /*Builder::new()
        .descriptor_pool("crate::api::DESCRIPTOR_POOL")
        .compile_protos_with_config(config, &protos, &[&proto_root])
        .expect("Failed to generate reflection data for services.");
*/
    Ok(())
}