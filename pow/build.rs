use std::path::PathBuf;
use normpath::PathExt;
use protogen::Generator;

fn main() -> std::io::Result<()> {
    built::write_built_file()?;

    let root_directory = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(dir) => PathBuf::from(dir).join("..").normalize()?,
        Err(_) => panic!("CARGO_MANIFEST_DIR is not set"),
    };

    #[cfg(windows)] let protoc = root_directory.join("./protoc.exe");
    #[cfg(not(windows))] let protoc = root_directory.join("./protoc");
    assert!(matches!(std::fs::exists(&protoc), Ok(true)), "protoc not found in {:?}", root_directory);

    let proto_root = root_directory.join("./protos");
    assert!(matches!(std::fs::exists(&proto_root), Ok(true)), "/protos/ subdirectory missing in {:?}", root_directory);

    macro_rules! make_abs {
        ($p:expr) => {
            proto_root.join($p)
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

    Generator::default()
        .build(protoc, &protos, &proto_root, root_directory.join("pow/src/protobuf"));

    Ok(())
}