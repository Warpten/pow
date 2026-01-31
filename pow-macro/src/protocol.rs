use heck::ToSnakeCase;
use proc_macro_error::abort;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{DeriveInput, Expr, Generics, Ident, Path, Token, bracketed, parse::{Parse, ParseStream}, parse2};

struct Protocol {
    identifier: CompleteType,
    handlers: Vec<Handler>,
}
struct Handler {
    ty: CompleteType,
    id: Expr,
    span: Span,
}

struct CompleteType {
    pub path: Path,
    pub generics: Generics,
}

impl ToTokens for CompleteType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.path.to_tokens(tokens);
        self.generics.to_tokens(tokens);
    }
}

impl Parse for CompleteType {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        // Try parsing a Path first.
        // This succeeds for:
        // - foo::bar::Baz
        // - Baz
        if let Ok(path) = input.parse::<Path>() {
            let generics = input.parse::<Generics>().ok().unwrap_or_default();

            return Ok(Self{ path, generics });
        }
        
        // If that fails, try parsing a bare Ident.
        if let Ok(ident) = input.parse::<Ident>() {
            let generics = input.parse::<Generics>().ok().unwrap_or_default();

            return Ok(Self{ path: ident.into(), generics });
        }
        
        Err(input.error("expected a type path or identifier"))
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
        let identifier: CompleteType = input.parse()?;
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

        // handler(type = ..., identifier = ...)
        input.parse::<handler>()?;
        let content;
        syn::parenthesized!(content in input);

        // type = <...>,
        content.parse::<ty>()?;
        content.parse::<Token![=]>()?;
        let ty: CompleteType = content.parse()?;
        
        content.parse::<Token![,]>()?;
        
        // identifier = <...>
        content.parse::<identifier>()?;
        content.parse::<Token![=]>()?;
        let id: Expr = content.parse()?;

        Ok(Handler { ty, id, span })
    }
}

pub fn derive_impl(attr: TokenStream, input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse2(input).expect("Failed to parse input");

    let struct_ident = CompleteType { path: input.ident.clone().into(), generics: input.generics.clone() };
    let struct_generics = &struct_ident.generics;

    let protocol = match syn::parse2::<Protocol>(attr) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };

    let identifier = protocol.identifier;
    let trait_ident = format_ident!("{}Implementation", struct_ident.path.get_ident().unwrap());

    let mut trait_methods = vec![];
    let mut match_arms = vec![];

    for handler in protocol.handlers {
        // Use the last segment of the path to obtain the function name
        let ty = handler.ty;
        let id = handler.id;
        let type_ident = match ty.path.segments.last() {
            Some(name) => name.ident.to_string(),
            None => abort!(handler.span, "Unable to determine a type name from {:?}", ty.path),
        };

        let method_ident = format_ident!("handle_{}", type_ident.to_snake_case());

        trait_methods.push(quote! {
            fn #method_ident<D>(&mut self, msg: #ty, dest: &mut D) -> impl ::core::future::Future<Output = anyhow::Result<()>>
                where D: ::pow_packets::WriteExt;
        });

        match_arms.push(quote! {
            #id => {
                let msg = <#ty as ::pow_packets::Payload>::recv(source, self).await?;
                <Self as #trait_ident>::#method_ident(self, msg, dest).await
            }
        });
    }

    let assertion = if struct_ident.generics == Default::default() {
        let assertion_ident = format_ident!("Assert{}", struct_ident.path.get_ident().unwrap());
        quote! {
             // Emit the assertion trait.
            trait #assertion_ident: #trait_ident {}
            impl<T: #trait_ident> #assertion_ident for T {}

            const _: fn() = || {
                fn assert_impl<__Type: #assertion_ident>() {}
                assert_impl::<#struct_ident>();
            };
        }
    } else {
        let assertion_ident = format_ident!("Assert{}", struct_ident.path.get_ident().unwrap());
        quote! {
             // Emit the assertion trait.
            trait #assertion_ident: #trait_ident {}
            impl<T: #trait_ident> #assertion_ident for T {}

            const _: for #struct_generics fn() = || {
                fn assert_impl<__Type: #assertion_ident>() {}
                assert_impl::<#struct_ident>();
            };
        }
    };

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
        impl #struct_generics ::pow_packets::Protocol for #struct_ident {
            fn process_incoming<S, D>(&mut self, source: &mut S, dest: &mut D) -> impl ::core::future::Future<Output = anyhow::Result<()>>
                where S : ::pow_packets::ReadExt, D: ::pow_packets::WriteExt
            {
                async {
                    let identifier = <#identifier as ::pow_packets::Identifier>::recv(source, self).await?;

                    match identifier {
                        #(#match_arms)*,
                        _ => Err(::anyhow::anyhow!("Unknown identifier"))
                    }
                }
            }
        }

        #assertion
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
            pub struct GruntProtocol<T>(u8, PhantomData<T>);
        };

        let output = derive_impl(attr, input);
        assert_tokens_eq!(output, quote!{
            pub struct GruntProtocol<T>(u8, PhantomData<T>);

            #[doc = "This trait expects a complete implementation of every handler associated with a protocol."]
            trait GruntProtocolImplementation {
                fn handle_logon_challenge_request<D>(&mut self, msg: LogonChallengeRequest, dest: &mut D)
                    -> impl ::core::future::Future<Output = anyhow::Result<()>>
                        where D: ::pow_packets::WriteExt;

                fn handle_logon_proof_request<D>(&mut self, msg: LogonProofRequest, dest: &mut D)
                    -> impl ::core::future::Future<Output = anyhow::Result<()>>
                        where D: ::pow_packets::WriteExt;
            }
            
            #[doc = "This implementation of Protocol was automatically generated."]
            impl<T> ::pow_packets::Protocol for GruntProtocol<T> {
                fn process_incoming<S, D>(&mut self, source: &mut S, dest: &mut D) -> impl ::core::future::Future<Output = anyhow::Result<()>>
                    where S: ::pow_packets::ReadExt, D: ::pow_packets::WriteExt
                {
                    async {
                        let identifier = <GruntIdentifier as ::pow_packets::Identifier>::recv(source, self).await?;
                        match identifier {
                            0x00 => {
                                let msg = <LogonChallengeRequest as ::pow_packets::Payload>::recv(source, self).await?;
                                <Self as GruntProtocolImplementation>::handle_logon_challenge_request(self, msg, dest).await
                            }
                            0x01 => {
                                let msg = <LogonProofRequest as ::pow_packets::Payload>::recv(source, self).await?;
                                <Self as GruntProtocolImplementation>::handle_logon_proof_request(self, msg, dest).await
                            }
                            _ => Err(::anyhow::anyhow!("Unknown identifier")),
                        }
                    }
                }
            }
            
            trait AssertGruntProtocol: GruntProtocolImplementation {}
            impl<T: GruntProtocolImplementation> AssertGruntProtocol for T {}
            
            const _: for<T> fn() = || {
                fn assert_impl<__Type: AssertGruntProtocol>() {}
                assert_impl::<GruntProtocol<T>>();
            };
        });
    }
}