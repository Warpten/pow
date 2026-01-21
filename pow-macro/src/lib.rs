use proc_macro::TokenStream;

mod enum_from;
mod enum_kind;

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
