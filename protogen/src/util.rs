use prost::Message;
use prost_reflect::{DynamicMessage, MethodDescriptor, ServiceDescriptor};

pub trait HasOptions {
    fn options(&self)  -> DynamicMessage;
}

impl HasOptions for MethodDescriptor {
    fn options(&self) -> DynamicMessage {
        MethodDescriptor::options(self)
    }
}

impl HasOptions for ServiceDescriptor {
    fn options(&self) -> DynamicMessage {
        ServiceDescriptor::options(self)
    }
}

pub fn find_extension<D: Message + Default, T: HasOptions>(data: &T, name: &str) -> Option<D> {
    data.options().extensions().find(|(extension, _)| {
        extension.full_name() == name
    }).map(|(_, value)| {
        value.as_message().map(|v| D::decode(v.encode_to_vec().as_slice()).ok()).flatten()
    }).flatten()
}