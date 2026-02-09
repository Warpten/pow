/// Contains all codegen for services.
use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use prost_reflect::{MethodDescriptor, ServiceDescriptor};
use quote::{format_ident, quote};
use crate::{DependencyInfo, MethodInfo};
use crate::proto::bgs::protocol::BgsMethodOptions;
use crate::util::find_extension;

#[inline]
pub(crate) fn render_methods_common<I>(items: I) -> (Vec<MethodInfo>, Vec<DependencyInfo>)
    where I : Iterator<Item = (MethodInfo, Vec<DependencyInfo>)>
{
    type Accumulator = (Vec<MethodInfo>, Vec<DependencyInfo>);
    type Item = (MethodInfo, Vec<DependencyInfo>);

    let acc = |(mut methods, imports): Accumulator, (method, dependencies): Item| {
        let imports = imports.into_iter()
            .chain(dependencies.into_iter())
            .collect();

        methods.push(method);
        (methods, imports)
    };

    items.fold((Vec::new(), Vec::new()), acc)
}

pub(crate) fn render_filter_methods<F>(svc: &ServiceDescriptor, function: F) -> (Vec<MethodInfo>, Vec<DependencyInfo>)
    where F: FnMut(MethodDescriptor) -> Option<(MethodInfo, Vec<DependencyInfo>)>
{
    render_methods_common(svc.methods().filter_map(function))
}

pub(crate) fn render_methods<F>(svc: &ServiceDescriptor, function: F) -> (Vec<MethodInfo>, Vec<DependencyInfo>)
    where F: FnMut(MethodDescriptor) -> (MethodInfo, Vec<DependencyInfo>)
{
    render_methods_common(svc.methods().map(function))
}

pub(crate) fn render_server_parse_block(md: MethodDescriptor) -> Option<TokenStream> {
    find_extension(&md, "bgs.protocol.method_options")
        .and_then(|o: BgsMethodOptions| o.id)
        .map(|method_id| {
            let input = DependencyInfo::from(&md.input()).ident();
            let handler_name = format_ident!("{}", md.name().to_snake_case());

            let handler_block = if md.output().name() == "NO_RESPONSE" {
                quote! { async { /* Nothing to do */ } }
            } else {
                quote! { self.send_response(dest, service, method, token, response) }
            };


            // Externally provided arguments: `method` and `token`.
            quote! {
                #method_id => match <#input>::parse(buffer) {
                    Ok(request) => match self.#handler_name(request) {
                        Ok(response) => #handler_block,
                        Err(status) => self.send_status(dest, method, token, status)
                    },
                    Err(_) => self.send_malformed_request(dest, method, token)
                }
            }
        })
}

pub(crate) fn render_client_handler(md: MethodDescriptor) -> (MethodInfo, Vec<DependencyInfo>) {
    let input = DependencyInfo::from(&md.input());
    let output = DependencyInfo::from(&md.output());

    let name = format_ident!("{}", md.name().to_snake_case());

    let input_name = input.ident();
    let output_name = output.ident();

    let method_id = find_extension(&md, "bgs.protocol.method_options")
        .and_then(|o: BgsMethodOptions| o.id)
        .expect("Unreachable code reached while generating an outbound handler");

    let implementation = if output.typename == "NO_RESPONSE" || output.typename == "NoData" {
        quote ! {
            fn #name(&mut self, msg: #input_name, client: bool, server: bool) -> impl Future<Output = Result<(), Self::ErrorCode>> {
                let mut method_id = #method_id;
                if client { method_id |= 0x40000000u32; }
                if server { method_id |= 0x80000000u32; }

                self.send(self.current_hash(), method_id, msg)
            }
        }
    } else {
        quote ! {
            fn #name(&mut self, msg: #input_name, client: bool, server: bool) -> impl Future<Output = Result<#output_name, Self::ErrorCode>> {
                let mut method_id = #method_id;
                if client { method_id |= 0x40000000u32; }
                if server { method_id |= 0x80000000u32; }

                // TODO: We need a callback mechanism here
                todo!("The callback mechanism is not implemented due to a lack of type erasure.");
                self.send(self.current_hash(), method_id, msg)
            }
        }
    };

    (MethodInfo {
        name,
        implementation,
    }, vec![input, output])
}

/// Renders a [`MethodDescriptor`] into a [`TokenStream`] as well as a collection of all packages
/// and types needed.
pub(crate) fn render_server_handler(md: MethodDescriptor) -> (MethodInfo, Vec<DependencyInfo>) {
    assert!(!md.is_client_streaming() && !md.is_server_streaming());

    let input = DependencyInfo::from(&md.input());
    let output = DependencyInfo::from(&md.output());

    let name = format_ident!("{}", md.name().to_snake_case());

    let input_name = input.ident();
    let output_name = output.ident();

    let implementation = if output.typename == "NO_RESPONSE" || output.typename == "NoData" {
        quote ! {
            fn #name(&mut self, msg: #input_name) -> impl Future<Output = Result<(), Self::ErrorCode>>;
        }
    } else {
        quote ! {
            fn #name(&mut self, msg: #input_name) -> impl Future<Output = Result<#output_name, Self::ErrorCode>>;
        }
    };

    (MethodInfo {
        name,
        implementation,
    }, vec![input, output])
}
