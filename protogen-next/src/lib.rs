use protobuf::proto;
use crate::protos::{BGSServiceOptions, FieldDescriptorProtoView, FileDescriptorProtoView, FileDescriptorSet, ServiceDescriptorProtoView, ServiceOptionsView};

#[allow(unused)]
mod protos {
    include!(concat!(env!("OUT_DIR"), "/protobuf_generated/bgs/low/pb/client/global_extensions/generated.rs"));
    include!(concat!(env!("OUT_DIR"), "/protobuf_generated/bgs/low/pb/client/generated.rs"));
    include!(concat!(env!("OUT_DIR"), "/protobuf_generated/bgs/low/pb/client/api/common/v1/generated.rs"));
}

fn main() {
    compile_error!("This tool is not usable yet. The official Rust crate for Protocol Buffers does not provide access to services nor extensions, which are required for this.");
}