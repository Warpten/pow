use heck::ToSnakeCase;
use proc_macro_error::abort;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{DeriveInput, Expr, Ident, Path, Token, bracketed, parse::{Parse, ParseStream}, parse2};

struct Protocol {
    identifier: TypePath,
    handlers: Vec<Handler>,
}
struct Handler {
    ty: TypePath,
    id: Expr,
    span: Span,
}

enum TypePath {
    Path(Path),
    Ident(Ident),
}

impl Parse for TypePath {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        // Try parsing a Path first.
        // This succeeds for:
        // - foo::bar::Baz
        // - Baz
        if let Ok(path) = input.parse::<Path>() {
            return Ok(TypePath::Path(path));
        }
        
        // If that fails, try parsing a bare Ident.
        if let Ok(ident) = input.parse::<Ident>() {
            return Ok(TypePath::Ident(ident));
        }
        
        Err(input.error("expected a type path or identifier"))
    }
}

impl TypePath {
    fn into_path(self) -> Path {
        match self {
            TypePath::Path(p) => p,
            TypePath::Ident(i) => Path::from(i),
        }
    }
}


syn::custom_keyword!(identifier);
syn::custom_keyword!(handlers);
syn::custom_keyword!(handler);
syn::custom_keyword!(ty);

impl Parse for Protocol {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        // identifier = GruntIdentifier,
        input.parse::<identifier>()?;
        input.parse::<Token![=]>()?;
        let identifier: TypePath = input.parse()?;
        input.parse::<Token![,]>()?;
        
        // handlers = [ handler(...), handler(...), ... ]
        input.parse::<handlers>()?; input.parse::<Token![=]>()?;
        
        let content;
        bracketed!(content in input);
        let handlers_punct = content.parse_terminated(Handler::parse, Token![,])?;
        let handlers = handlers_punct.into_iter().collect();
        Ok(Protocol { identifier, handlers })
    }
}

impl Parse for Handler {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let span = input.span();

        // handler(type = LogonChallengeRequest, identifier = ...)
        input.parse::<handler>()?;
        let content;
        syn::parenthesized!(content in input);

        // type = <Path>,
        content.parse::<ty>()?;
        content.parse::<Token![=]>()?;
        let ty: TypePath = content.parse()?;
        
        content.parse::<Token![,]>()?;
        
        // identifier = <Expr>
        content.parse::<identifier>()?;
        content.parse::<Token![=]>()?;
        let id: Expr = content.parse()?;

        Ok(Handler { ty, id, span })
    }
}

pub fn derive_impl(attr: TokenStream, input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse2(input).expect("Failed to parse input");

    let struct_ident = input.ident.clone();
    let vis = input.vis.clone();

    let protocol = match syn::parse2::<Protocol>(attr) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };

    let identifier_ty = protocol.identifier.into_path();

    let trait_ident = format_ident!("{}Implementation", input.ident);

    let mut trait_methods = vec![];
    let mut match_arms = vec![];

    for handler in protocol.handlers {
        // Use the last segment of the path to obtain the function name
        let ty = handler.ty.into_path();
        let id = handler.id;
        let type_ident = match ty.segments.last() {
            Some(name) => name.ident.to_string(),
            None => abort!(handler.span, "Unable to determine a type name from {}", ty.to_token_stream()),
        };

        let method_ident = format_ident!("handle_{}", type_ident.to_snake_case());

        trait_methods.push(quote! {
            fn #method_ident(&mut self, msg: #ty) -> impl ::core::future::Future<Output = anyhow::Result<()>>;
        });

        match_arms.push(quote! {
            #id => {
                let msg = <#ty as ::pow_packets::Payload>::recv(source, self).await?;
                <Self as #trait_ident>::#method_ident(self, msg).await
            }
        });
    }

    let assertion_ident = format_ident!("Assert{}", struct_ident);

    quote! {
        // Re-paste the input
        #input

        // Emit the trait itself.

        #[doc = "This trait expects a complete implementation of every handler associated with a protocol."]
        trait #trait_ident {
            #(#trait_methods)*
        }

        // Emit the Protocol implementation.

        #[doc = "This implementation of Protocol was automatically generated."]
        impl ::pow_packets::Protocol for #struct_ident {
            fn process_incoming<S>(&mut self, source: &mut S) -> impl ::core::future::Future<Output = anyhow::Result<()>>
                where S : ::pow_packets::ReadExt
            {
                async {
                    let identifier = <#identifier_ty as ::pow_packets::Identifier>::recv(source, self).await?;

                    match identifier {
                        #(#match_arms)*,
                        _ => Err(::anyhow::anyhow!("Unknown identifier"))
                    }
                }
            }
        }

        // Emit the assertion trait.
        trait #assertion_ident: #trait_ident {}
        impl<T: #trait_ident> #assertion_ident for T {}

        const _: fn() = || {
            fn assert_impl<T: #assertion_ident>() {}
            assert_impl::<#struct_ident>();
        };
    }.into()
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use assert_tokens_eq::assert_tokens_eq;
    use crate::protocol::derive_impl;

    #[test]
    pub fn test_derive() {
        let attr = quote! {
            identifier = GruntIdentifier, handlers = [
                handler(ty = LogonChallengeRequest, identifier = 0x00),
                handler(ty = LogonProofRequest, identifier = 0x01)
            ]
        };

        let input = quote! {
            pub struct GruntProtocol(u8);
        };

        let output = derive_impl(attr, input);
        assert_tokens_eq!(output, quote!{
            pub struct GruntProtocol(u8);

            #[doc = "This trait expects a complete implementation of every handler associated with a protocol."]
            trait GruntProtocolImplementation {
                fn handle_logon_challenge_request(
                    &mut self,
                    msg: LogonChallengeRequest,
                ) -> impl ::core::future::Future<Output = anyhow::Result<()>>;
                fn handle_logon_proof_request(
                    &mut self,
                    msg: LogonProofRequest,
                ) -> impl ::core::future::Future<Output = anyhow::Result<()>>;
            }
            
            #[doc = "This implementation of Protocol was automatically generated."]
            impl ::pow_packets::Protocol for GruntProtocol {
                fn process_incoming<S>(&mut self, source: &mut S) -> impl ::core::future::Future<Output = anyhow::Result<()>>
                where
                    S: ::pow_packets::ReadExt,
                {
                    async {
                        let identifier = <GruntIdentifier as ::pow_packets::Identifier>::recv(source, self).await?;
                        match identifier {
                            0x00 => {
                                let msg = <LogonChallengeRequest as ::pow_packets::Payload>::recv(source, self).await?;
                                <Self as GruntProtocolImplementation>::handle_logon_challenge_request(self, msg).await
                            }
                            0x01 => {
                                let msg = <LogonProofRequest as ::pow_packets::Payload>::recv(source, self).await?;
                                <Self as GruntProtocolImplementation>::handle_logon_proof_request(self, msg).await
                            }
                            _ => Err(::anyhow::anyhow!("Unknown identifier")),
                        }
                    }
                }
            }
            
            trait AssertGruntProtocol: GruntProtocolImplementation {}
            impl<T: GruntProtocolImplementation> AssertGruntProtocol for T {}
            
            const _: fn() = || {
                fn assert_impl<T: AssertGruntProtocol>() {}
                assert_impl::<GruntProtocol>();
            };
        });
    }
}