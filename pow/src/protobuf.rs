use protobuf::proto;

mod protos {
    include!(concat!(env!("OUT_DIR"), "/protobuf_generated/account_service.rs")) ;
    include!(concat!(env!("OUT_DIR"), "/protobuf_generated/account_listener.rs")) ;

    // include!(concat!(env!("OUT_DIR"), "/protobuf_generated/bgs/low/pb/client/global_extensions/generated.rs"));
    include!(concat!(env!("OUT_DIR"), "/protobuf_generated/bgs/low/pb/client/generated.rs"));
    // include!(concat!(env!("OUT_DIR"), "/protobuf_generated/bgs/low/pb/client/api/common/v1/generated.rs"));
}