use proc_macro::TokenStream;

mod enum_from;
mod enum_kind;
mod protocol;

#[proc_macro_derive(EnumFrom)]
pub fn derive_enum_from(input: TokenStream) -> TokenStream {
    let result = enum_from::enum_from_impl(proc_macro2::TokenStream::from(input));

    TokenStream::from(result)
}

#[proc_macro_derive(EnumKind, attributes(kind))]
pub fn derive_enum_kind(input: TokenStream) -> TokenStream {
    let result = enum_kind::enum_kind_impl(proc_macro2::TokenStream::from(input));

    TokenStream::from(result)
}

#[cfg(not(doctest))]
/// Implements the [`Protocol`] trait for this type and generates
/// a trait with a function for each packet type declared in the handlers.
/// 
/// Example usage:
/// 
/// ```
/// use crate::grunt::protocol::{self};
/// 
/// #[protocol(identifier = GruntIdentifier, handlers = [
///     handler(ty = LogonChallengeRequest, identifier = GruntIdentifier(0x00)),
///     handler(ty = LogonProofRequest, identifier = GruntIdentifier(0x01))
/// ])]
/// ```
#[proc_macro_attribute]
pub fn protocol(attr: TokenStream, input: TokenStream) -> TokenStream {
    let result = protocol::derive_impl(attr.into(), input.into());

    TokenStream::from(result)
}

