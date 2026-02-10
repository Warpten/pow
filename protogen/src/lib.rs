#![allow(deprecated, unused_assignments, dead_code)]
#![allow(clippy::deprecated)]

mod util;
mod service;

use crate::proto::bgs::protocol::{BgsServiceOptions, SdkServiceOptions};
use crate::util::find_extension;
use heck::ToSnakeCase;
use itertools::Itertools;
use proc_macro2::{Literal, TokenStream};
use prost_reflect::prost_types::FileDescriptorSet;
use prost_reflect::{FileDescriptor, MessageDescriptor, ServiceDescriptor};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use std::hash::Hash;
use std::str::FromStr;
use protobuf_codegen::CodeGen;
use syn::{parse_str, Ident, Path};
use crate::proto::DESCRIPTOR_POOL;

mod proto {
    use std::sync::LazyLock;
    use prost::Message;
    use prost_reflect::DescriptorPool;
    use prost_reflect::prost_types::FileDescriptorSet;

    pub static DESCRIPTOR_POOL: LazyLock<DescriptorPool> = LazyLock::new(|| DescriptorPool::from_file_descriptor_set(
        FileDescriptorSet::decode(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fd.bin")).as_ref())
            .expect("FileDescriptorSet decode failed")
    ).unwrap());

    include!("proto/proto.rs");
}

struct MethodInfo {
    name: Ident,
    /// The method. This can either be its declaration (eg a trait method) or its implementation.
    implementation: TokenStream,
}

impl ToTokens for MethodInfo {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.implementation.clone());
    }
}

fn hash(data: &[u8]) -> u32 {
    data.iter()
        .fold(0x811C9DC5, |acc, c| (acc ^ (*c as u32)).wrapping_mul(0x1000193))
}

/// Holds a type as well as the package that contains it.
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Clone)]
struct DependencyInfo {
    base: String,

    /// The complete path where this type lives.
    path: String,

    /// The name of this type.
    typename: String
}

impl DependencyInfo {
    pub fn path(&self) -> Path {
        parse_str(format!("{}::{}", self.base, self.path).as_str()).unwrap()
    }

    pub fn rebase<S: Into<String>>(mut self, base: S) -> Self {
        self.base = base.into();
        self
    }

    pub fn try_rebase<S: Into<String>>(self, base: Option<S>) -> Self {
        if let Some(base) = base {
            self.rebase(base)
        } else {
            self
        }
    }

    pub fn ident(&self) -> Ident {
        parse_str(self.typename.as_str()).unwrap()
    }

    pub fn from(desc: &MessageDescriptor) -> Self {
        Self {
            base: "crate".to_string(),
            path: desc.package_name().to_string(),
            typename: desc.name().to_string(),
        }
    }

    pub fn from_path(path: &str) -> Self {
        let tokens : [&str; 2] = path.rsplitn(2, ".")
            .collect_array()
            .expect("Should match");

        Self {
            base: "crate".to_string(),
            path: tokens[0].replace(".", "::").to_string(),
            typename: tokens[1].to_string()
        }
    }
}

impl ToTokens for DependencyInfo {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let base = parse_str::<Path>(self.base.replace(".", "::").as_str())
            .expect("invalid base path");
        let path = parse_str::<Path>(self.path.replace(".", "::").as_str())
            .expect("invalid path");
        let name = format_ident!("{}", &self.typename);

        tokens.append_all(quote! {
            use #base::#path::#name;
        });
    }
}

#[derive(Default)]
pub struct Generator {
    fds: Vec<FileDescriptorSet>,
    path: Option<String>,
}

impl Generator {
    pub fn new<C: Into<Vec<FileDescriptorSet>>>(fds: C) -> Self {
        Self { fds: fds.into(), ..Default::default() }
    }

    /// Sets a crate path to override the root of the path to generated types.
    /// By default, service implementation will live in `crate::package::path`,
    /// where `package::path` maps to the package name in the declaration of
    /// said service.
    pub fn crate_path<S: Into<String>>(&mut self, path: S) -> &mut Self {
        self.path = Some(path.into());
        self
    }

    fn generate_protobuf_types(
        &self,
        protoc: impl AsRef<std::path::Path>,
        protos: impl IntoIterator<Item = impl AsRef<std::path::Path>>,
        include: impl AsRef<std::path::Path>,
        output: impl AsRef<std::path::Path>
    ) {
        // Use the Protobuf generator crate
        CodeGen::new()
            .protoc_path(protoc)
            // Get the path to all proto files.
            .include(include)
            .inputs(protos)
            .output_dir(output)
            .dependency(protobuf_well_known_types::get_dependency("protobuf_well_known_types"))
            .generate_and_compile()
            .expect("Failed to generate Protobuf messages, enums, and extensions.");
    }

    pub fn build(&self, protoc: impl AsRef<std::path::Path>,
                 protos: impl IntoIterator<Item = impl AsRef<std::path::Path>>,
                 include: impl AsRef<std::path::Path>,
                 output: impl AsRef<std::path::Path>) {
        self.generate_protobuf_types(protoc, protos, include, output);

        // Now, generate services.
        let base_output_dir = std::env::var("OUT_DIR")
            .expect("Failed to obtain OUT_DIR");

        // Read reflection data for services in the descriptor pool and generate the services.
        DESCRIPTOR_POOL.services()
            .map(|svc| {
                println!("Rendering {}", svc.full_name());
                let r#impl = self.render_service(&svc);

                let dep = DependencyInfo::from_path(svc.full_name())
                    .try_rebase(self.path.as_ref());

                (dep, r#impl)
            })
            .for_each(|(path, contents)| {
                let contents = format!("{}", contents);

                let relative_path = format!("/protobuf_generated/{}.rs", path.path.replace("::", "/").to_snake_case());
                let path = format!("{}{}", base_output_dir, relative_path);

                // Format the file.
                let contents = prettyplease::unparse(&syn::parse_file(contents.as_str()).expect("Failed to read unformatted code"));
                std::fs::write(path, contents)
                    .expect("Failed to write formatted code");
            });
    }

    fn render_file_descriptor(&self, fd: &FileDescriptor) -> TokenStream {
        let services = fd.services().map(|svc| self.render_service(&svc));

        quote! {
            // Generated by protogen. Do not edit.
            #(#services)*
        }
    }

    fn render_service(&self, svc: &ServiceDescriptor) -> TokenStream {
        let name = format_ident!("{}", svc.name());
        let full_name = svc.full_name();

        let make_token_hash= |name: &[u8]| {
            let hash = hash(name);
            Literal::from_str(
                &format!("0x{:X}u32", hash)
            ).unwrap()
        };

        let sdk_service_options: Option<SdkServiceOptions> = find_extension(svc, "bgs.protocol.sdk_service_options");

        // Extract the hashes for this service.
        let hashes = find_extension(svc, "bgs.protocol.service_options")
            .map(|extension: BgsServiceOptions| {
                let name_hash = make_token_hash(full_name.as_bytes());
                let hash = make_token_hash(extension.descriptor_name.expect("Missing descriptor name").as_bytes());

                quote! {
                    const ORIGINAL_HASH: u32 = #hash;
                    const NAME_HASH: u32 = #name_hash;

                    #[doc = "Returns the currently active hash."]
                    fn current_hash(&self) -> u32;
                }
            }).unwrap_or_else(|| {
                let name_hash = make_token_hash(full_name.as_bytes());

                quote! {
                    const NAME_HASH: u32 = #name_hash;

                    #[doc = "Returns the currently active hash."]
                    fn current_hash(&self) -> u32 { NAME_HASH }
                }
            });

        let outbound = sdk_service_options.and_then(|o| o.outbound).unwrap_or(false);
        let inbound = sdk_service_options.and_then(|o| o.inbound).unwrap_or(false);

        // Extract client interface
        let (client_imports, client) = if inbound {
            let (methods, imports) = service::render_methods(&svc, service::render_client_handler);

            (imports, quote! {
                #(#methods)*
            })
        } else {
            (Vec::new(), quote! { })
        };

        // Extract server interface
        let (server_imports, server) = if outbound {
            let (methods, imports) = service::render_methods(&svc, service::render_server_handler);

            // Generate parsers and handler calls.
            let parse_blocks: Vec<_> = svc.methods()
                .filter_map(service::render_server_parse_block)
                .collect();

            (imports, quote! {
                #(#methods)*

                fn call_server_method<Peer>(&mut self, connection: &mut Peer, token: u32, method: u32, payload: &[u8])
                    -> impl Future<Output = ()>
                {
                    match (method & 0x3FFFFFFFu32) {
                        #(#parse_blocks),*
                        _ => self.send_status(connection, method, token, ERROR_RPC_INVALID_METHOD)
                    }
                }

                fn send_status<Peer>(&mut self, connection: &mut Peer, method: u32, token: u32, status: Self::ErrorCode) -> impl Future<Output = ()>;

                fn send_response<Peer, T>(&mut self, connection: &mut Peer, method: u32, token: u32, message: T) -> impl Future<Output = ()>;

                fn send_malformed_request<Peer>(&mut self, connection: &mut Peer, method: u32, token: u32) -> impl Future<Output = ()>;
                fn send_invalid_method<Peer>(&mut self, connection: &mut Peer, method: u32, token: u32) -> impl Future<Output = ()>;
            })
        } else {
            (Vec::new(), quote! { })
        };

        // Collect imports, sort them, and make them unique.
        let imports = server_imports.into_iter()
            .chain(client_imports)
            .sorted()
            .unique()
            .map(|dep| dep.try_rebase(self.path.as_ref()))
            .collect::<Vec<_>>();

        // Emit the entire service along with its dependencies.
        quote! {
            use protobuf::prelude::*;
            #(#imports)*

            pub trait #name {
                #hashes

                #[doc = "The error code that RPC calls may return."]
                type ErrorCode;

                #[doc = "A predefined error code to identify invalid methods."]
                const ERROR_RPC_INVALID_METHOD: Self::ErrorCode;

                #[doc = "A predefined error code to identify non-implemented methods."]
                const ERROR_RPC_NOT_IMPLEMENTED: Self::ErrorCode;

                #client
                #server
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::proto::DESCRIPTOR_POOL;
    use crate::Generator;

    use assert_tokens_eq::assert_tokens_eq;
    use quote::quote;

    #[test]
    pub fn test_service() {
        let fd = DESCRIPTOR_POOL.files()
            .find(|f| f.name() == "bgs/low/pb/client/account_service.proto")
            .expect("Could not find account_service.proto");

        // Generate all services.
        let outcome = Generator::default()
            .render_file_descriptor(&fd);

        assert_tokens_eq! { outcome, quote! {
            use crate::bgs::protocol::NoData;
            use crate::bgs::protocol::account::v1::GetAccountInfoRequest;
            use crate::bgs::protocol::account::v1::GetAccountInfoResponse;
            use crate::bgs::protocol::account::v1::GetAccountPlatformRestrictionsRequest;
            use crate::bgs::protocol::account::v1::GetAccountPlatformRestrictionsResponse;
            use crate::bgs::protocol::account::v1::GetAccountStateRequest;
            use crate::bgs::protocol::account::v1::GetAccountStateResponse;
            use crate::bgs::protocol::account::v1::GetAuthorizedDataRequest;
            use crate::bgs::protocol::account::v1::GetAuthorizedDataResponse;
            use crate::bgs::protocol::account::v1::GetCAISInfoRequest;
            use crate::bgs::protocol::account::v1::GetCAISInfoResponse;
            use crate::bgs::protocol::account::v1::GetGameAccountStateRequest;
            use crate::bgs::protocol::account::v1::GetGameAccountStateResponse;
            use crate::bgs::protocol::account::v1::GetGameSessionInfoRequest;
            use crate::bgs::protocol::account::v1::GetGameSessionInfoResponse;
            use crate::bgs::protocol::account::v1::GetGameTimeRemainingInfoRequest;
            use crate::bgs::protocol::account::v1::GetGameTimeRemainingInfoResponse;
            use crate::bgs::protocol::account::v1::GetLicensesRequest;
            use crate::bgs::protocol::account::v1::GetLicensesResponse;
            use crate::bgs::protocol::account::v1::GetSignedAccountStateRequest;
            use crate::bgs::protocol::account::v1::GetSignedAccountStateResponse;
            use crate::bgs::protocol::account::v1::ResolveAccountRequest;
            use crate::bgs::protocol::account::v1::ResolveAccountResponse;
            use crate::bgs::protocol::account::v1::SubscriptionUpdateRequest;
            use crate::bgs::protocol::account::v1::SubscriptionUpdateResponse;
            use protobuf::prelude::*;

            pub trait AccountService {
                const ORIGINAL_HASH: u32 = 0x62DA0891u32;
                const NAME_HASH: u32 = 0x1E4DC42Fu32;

                #[doc = "Returns the currently active hash."]
                fn current_hash(&self) -> u32;

                #[doc = "The error code that RPC calls may return."]
                type ErrorCode;

                #[doc = "A predefined error code to identify invalid methods."]
                const ERROR_RPC_INVALID_METHOD: Self::ErrorCode;

                #[doc = "A predefined error code to identify non-implemented methods."]
                const ERROR_RPC_NOT_IMPLEMENTED: Self::ErrorCode;

                fn resolve_account(&mut self, msg: ResolveAccountRequest,)
                    -> impl Future<Output = Result<ResolveAccountResponse, Self::ErrorCode>>;

                fn subscribe(&mut self, msg: SubscriptionUpdateRequest,)
                    -> impl Future<Output = Result<SubscriptionUpdateResponse, Self::ErrorCode>>;

                fn unsubscribe(&mut self, msg: SubscriptionUpdateRequest,)
                    -> impl Future<Output = Result<(), Self::ErrorCode>>;

                fn get_account_state(&mut self, msg: GetAccountStateRequest,)
                    -> impl Future<Output = Result<GetAccountStateResponse, Self::ErrorCode>>;

                fn get_game_account_state(&mut self, msg: GetGameAccountStateRequest,)
                    -> impl Future<Output = Result<GetGameAccountStateResponse, Self::ErrorCode>>;

                fn get_licenses(&mut self, msg: GetLicensesRequest)
                    -> impl Future<Output = Result<GetLicensesResponse, Self::ErrorCode>>;

                fn get_game_time_remaining_info(&mut self, msg: GetGameTimeRemainingInfoRequest,)
                    -> impl Future<Output = Result<GetGameTimeRemainingInfoResponse, Self::ErrorCode>>;

                fn get_game_session_info(&mut self, msg: GetGameSessionInfoRequest,)
                    -> impl Future<Output = Result<GetGameSessionInfoResponse, Self::ErrorCode>>;

                fn get_cais_info(&mut self, msg: GetCAISInfoRequest,)
                    -> impl Future<Output = Result<GetCAISInfoResponse, Self::ErrorCode>>;

                fn get_authorized_data(&mut self, msg: GetAuthorizedDataRequest,)
                    -> impl Future<Output = Result<GetAuthorizedDataResponse, Self::ErrorCode>>;

                fn get_signed_account_state(&mut self, msg: GetSignedAccountStateRequest,)
                    -> impl Future<Output = Result<GetSignedAccountStateResponse, Self::ErrorCode>>;

                fn get_account_info(&mut self, msg: GetAccountInfoRequest,)
                    -> impl Future<Output = Result<GetAccountInfoResponse, Self::ErrorCode>>;

                fn get_account_platform_restrictions(&mut self, msg: GetAccountPlatformRestrictionsRequest,)
                    -> impl Future<Output = Result<GetAccountPlatformRestrictionsResponse, Self::ErrorCode>>;

                pub fn call_server_method<Peer>(&mut self, connection: &mut Peer, token: u32, method: u32, payload: &[u8],)
                    -> impl Future<Output = ()>
                {
                    match (method & 0x3FFFFFFFu32) {
                        13u32 => match <ResolveAccountRequest>::parse(buffer) {
                            Ok(request) => match self.resolve_account(request) {
                                Ok(response) => self.send_response(dest, service, method, token, response),
                                Err(status) => self.send_status(dest, method, token, status),
                            },
                            Err(_) => self.send_malformed_request(dest, method, token),
                        },
                        25u32 => match <SubscriptionUpdateRequest>::parse(buffer) {
                            Ok(request) => match self.subscribe(request) {
                                Ok(response) => self.send_response(dest, service, method, token, response),
                                Err(status) => self.send_status(dest, method, token, status),
                            },
                            Err(_) => self.send_malformed_request(dest, method, token),
                        },
                        26u32 => match <SubscriptionUpdateRequest>::parse(buffer) {
                            Ok(request) => match self.unsubscribe(request) {
                                Ok(response) => self.send_response(dest, service, method, token, response),
                                Err(status) => self.send_status(dest, method, token, status),
                            },
                            Err(_) => self.send_malformed_request(dest, method, token),
                        },
                        30u32 => match <GetAccountStateRequest>::parse(buffer) {
                            Ok(request) => match self.get_account_state(request) {
                                Ok(response) => self.send_response(dest, service, method, token, response),
                                Err(status) => self.send_status(dest, method, token, status),
                            },
                            Err(_) => self.send_malformed_request(dest, method, token),
                        },
                        31u32 => match <GetGameAccountStateRequest>::parse(buffer) {
                            Ok(request) => match self.get_game_account_state(request) {
                                Ok(response) => self.send_response(dest, service, method, token, response),
                                Err(status) => self.send_status(dest, method, token, status),
                            },
                            Err(_) => self.send_malformed_request(dest, method, token),
                        },
                        32u32 => match <GetLicensesRequest>::parse(buffer) {
                            Ok(request) => match self.get_licenses(request) {
                                Ok(response) => self.send_response(dest, service, method, token, response),
                                Err(status) => self.send_status(dest, method, token, status),
                            },
                            Err(_) => self.send_malformed_request(dest, method, token),
                        },
                        33u32 => match <GetGameTimeRemainingInfoRequest>::parse(buffer) {
                            Ok(request) => match self.get_game_time_remaining_info(request) {
                                Ok(response) => self.send_response(dest, service, method, token, response),
                                Err(status) => self.send_status(dest, method, token, status),
                            },
                            Err(_) => self.send_malformed_request(dest, method, token),
                        },
                        34u32 => match <GetGameSessionInfoRequest>::parse(buffer) {
                            Ok(request) => match self.get_game_session_info(request) {
                                Ok(response) => self.send_response(dest, service, method, token, response),
                                Err(status) => self.send_status(dest, method, token, status),
                            },
                            Err(_) => self.send_malformed_request(dest, method, token),
                        },
                        35u32 => match <GetCAISInfoRequest>::parse(buffer) {
                            Ok(request) => match self.get_cais_info(request) {
                                Ok(response) => self.send_response(dest, service, method, token, response),
                                Err(status) => self.send_status(dest, method, token, status),
                            },
                            Err(_) => self.send_malformed_request(dest, method, token),
                        },
                        37u32 => match <GetAuthorizedDataRequest>::parse(buffer) {
                            Ok(request) => match self.get_authorized_data(request) {
                                Ok(response) => self.send_response(dest, service, method, token, response),
                                Err(status) => self.send_status(dest, method, token, status),
                            },
                            Err(_) => self.send_malformed_request(dest, method, token),
                        },
                        44u32 => match <GetSignedAccountStateRequest>::parse(buffer) {
                            Ok(request) => match self.get_signed_account_state(request) {
                                Ok(response) => self.send_response(dest, service, method, token, response),
                                Err(status) => self.send_status(dest, method, token, status),
                            },
                            Err(_) => self.send_malformed_request(dest, method, token),
                        },
                        45u32 => match <GetAccountInfoRequest>::parse(buffer) {
                            Ok(request) => match self.get_account_info(request) {
                                Ok(response) => self.send_response(dest, service, method, token, response),
                                Err(status) => self.send_status(dest, method, token, status),
                            },
                            Err(_) => self.send_malformed_request(dest, method, token),
                        },
                        46u32 => match <GetAccountPlatformRestrictionsRequest>::parse(buffer) {
                            Ok(request) => match self.get_account_platform_restrictions(request) {
                                Ok(response) => self.send_response(dest, service, method, token, response),
                                Err(status) => self.send_status(dest, method, token, status),
                            },
                            Err(_) => self.send_malformed_request(dest, method, token),
                        },
                        _ => self.send_status(connection, method, token, ERROR_RPC_INVALID_METHOD),
                    }
                }

                fn send_status<Peer>(&mut self, connection: &mut Peer, method: u32, token: u32, status: Self::ErrorCode,)
                    -> impl Future<Output = ()>;

                fn send_response<Peer, T>(&mut self, connection: &mut Peer, method: u32, token: u32, message: T,)
                    -> impl Future<Output = ()>;

                fn send_malformed_request<Peer>(&mut self, connection: &mut Peer, method: u32, token: u32,)
                    -> impl Future<Output = ()>;

                fn send_invalid_method<Peer>(&mut self, connection: &mut Peer, method: u32, token: u32,)
                    -> impl Future<Output = ()>;
            }

            use crate::bgs::protocol::NO_RESPONSE;
            use crate::bgs::protocol::account::v1::AccountStateNotification;
            use crate::bgs::protocol::account::v1::GameAccountNotification;
            use crate::bgs::protocol::account::v1::GameAccountSessionNotification;
            use crate::bgs::protocol::account::v1::GameAccountStateNotification;
            use protobuf::prelude::*;

            pub trait AccountListener {
                const ORIGINAL_HASH: u32 = 0x54DFDA17u32;
                const NAME_HASH: u32 = 0x7807483Cu32;

                #[doc = "Returns the currently active hash."]
                fn current_hash(&self) -> u32;

                #[doc = "The error code that RPC calls may return."]
                type ErrorCode;

                #[doc = "A predefined error code to identify invalid methods."]
                const ERROR_RPC_INVALID_METHOD: Self::ErrorCode;

                #[doc = "A predefined error code to identify non-implemented methods."]
                const ERROR_RPC_NOT_IMPLEMENTED: Self::ErrorCode;

                fn on_account_state_updated(&mut self, msg: AccountStateNotification, client: bool, server: bool,)
                    -> impl Future<Output = Result<(), Self::ErrorCode>>
                {
                    let mut method_id = 1u32;
                    if client { method_id |= 0x40000000u32; }
                    if server { method_id |= 0x80000000u32; }
                    self.send(self.current_hash(), method_id, msg)
                }

                fn on_game_account_state_updated(&mut self, msg: GameAccountStateNotification, client: bool, server: bool,)
                    -> impl Future<Output = Result<(), Self::ErrorCode>> {
                    let mut method_id = 2u32;
                    if client { method_id |= 0x40000000u32; }
                    if server { method_id |= 0x80000000u32; }
                    self.send(self.current_hash(), method_id, msg)
                }

                fn on_game_accounts_updated(&mut self, msg: GameAccountNotification, client: bool, server: bool,)
                    -> impl Future<Output = Result<(), Self::ErrorCode>>
                {
                    let mut method_id = 3u32;
                    if client { method_id |= 0x40000000u32; }
                    if server { method_id |= 0x80000000u32; }
                    self.send(self.current_hash(), method_id, msg)
                }

                fn on_game_session_updated(&mut self, msg: GameAccountSessionNotification, client: bool, server: bool,)
                    -> impl Future<Output = Result<(), Self::ErrorCode>>
                {
                    let mut method_id = 4u32;
                    if client { method_id |= 0x40000000u32; }
                    if server { method_id |= 0x80000000u32; }
                    self.send(self.current_hash(), method_id, msg)
                }
            }
        } };
    }
}