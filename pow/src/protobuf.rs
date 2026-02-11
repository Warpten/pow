mod protos {
    include!("protobuf/bgs/low/pb/client/generated.rs");
    
    include!("protobuf/bgs/protocol/account/v1/mod.pb.rs");
    include!("protobuf/bgs/protocol/authentication/v1/mod.pb.rs");
    include!("protobuf/bgs/protocol/block_list/v1/client/mod.pb.rs");
    include!("protobuf/bgs/protocol/challenge/v1/mod.pb.rs");
    include!("protobuf/bgs/protocol/club_membership/v1/client/mod.pb.rs");
    include!("protobuf/bgs/protocol/connection/v1/mod.pb.rs");
    include!("protobuf/bgs/protocol/friends/v1/mod.pb.rs");
    include!("protobuf/bgs/protocol/game_utilities/v1/mod.pb.rs");
    include!("protobuf/bgs/protocol/notification/v2/client/mod.pb.rs");
    include!("protobuf/bgs/protocol/presence/v1/mod.pb.rs");
    include!("protobuf/bgs/protocol/report/v2/mod.pb.rs");
    include!("protobuf/bgs/protocol/resources/v1/mod.pb.rs");
    include!("protobuf/bgs/protocol/whisper/v2/client/mod.pb.rs");
}