use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, Ident, ItemTrait, Token, TraitItem, TraitItemFn, bracketed, parse::{Parse, ParseStream}, parse2};

struct Protocol {
    identifier: Ident,
    handlers: Vec<Handler>,
}
struct Handler {
    ty: Ident,
    id: Expr,
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
        let identifier: Ident = input.parse()?;
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
        // handler(type = ..., identifier = ...)
        input.parse::<handler>()?;
        let content;
        syn::parenthesized!(content in input);

        // type = <...>,
        content.parse::<ty>()?;
        content.parse::<Token![=]>()?;
        let ty: Ident = content.parse()?;
        
        content.parse::<Token![,]>()?;
        
        // identifier = <...>
        content.parse::<identifier>()?;
        content.parse::<Token![=]>()?;
        let id: Expr = content.parse()?;

        Ok(Handler { ty, id })
    }
}

pub fn derive_impl(attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut input: ItemTrait = syn::parse2(input).expect("[protocol(...)] can only be applied to traits.");

    let protocol = match syn::parse2::<Protocol>(attr) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };

    let mut match_arms = vec![];

    for handler in protocol.handlers {
        let ty = handler.ty; // The packet type.
        let id = handler.id; // The packet identifier.

        let handler_ident = format_ident!("handle_{}", ty.to_string().to_snake_case());

        let handler: TraitItemFn = {
            let quoted = quote! {
                fn #handler_ident<D>(&mut self, msg: #ty, dest: &mut D)
                    -> impl ::core::future::Future<Output = anyhow::Result<()>> + Send
                        where D: crate::packets::WriteExt;
            };

            parse2(quoted).expect("Failed to parse method handler.")
        };

        input.items.push(TraitItem::Fn(handler));

        match_arms.push(quote! {
            #id => {
                let msg = <#ty as crate::packets::Payload<T>>::recv(source, self).await?;
                Self::#handler_ident(self, msg, dest).await
            }
        })
    }

    let trait_ident = &input.ident;
    let identifier_ty = &protocol.identifier;

    quote! {
        #input

        // Emit the blanket implementation
        impl<T> Protocol for T where T: #trait_ident {
            fn process_incoming<Source, Dest>(&mut self, source: &mut Source, dest: &mut Dest)
                -> impl ::core::future::Future<Output = anyhow::Result<()>> + Send
                    where Source: crate::packets::ReadExt, Dest: crate::packets::WriteExt
            {
                async move {
                    let identifier = <#identifier_ty as crate::packets::Identifier<T>>::recv(source, self).await?;

                    match identifier {
                        #(#match_arms)*,
                        _ => Err(::anyhow::anyhow!("Unknown identifier"))
                    }
                }
            }
        }
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
            pub trait GruntProtocol { }
        };

        let output = derive_impl(attr, input);
        assert_tokens_eq!(output, quote!{
            pub trait GruntProtocol {
                fn handle_logon_challenge_request<D>(&mut self, msg: LogonChallengeRequest, dest: &mut D)
                    -> impl ::core::future::Future<Output = anyhow::Result<()>> + Send
                        where D: crate::packets::WriteExt;

                fn handle_logon_proof_request<D>(&mut self, msg: LogonProofRequest, dest: &mut D)
                    -> impl ::core::future::Future<Output = anyhow::Result<()>> + Send
                        where D: crate::packets::WriteExt;
            }

            impl<T> Protocol for T where T: GruntProtocol {
                fn process_incoming<Source, Dest>(&mut self, source: &mut Source, dest: &mut Dest)
                    -> impl ::core::future::Future<Output = anyhow::Result<()>> + Send
                        where Source: crate::packets::ReadExt, Dest: crate::packets::WriteExt,
                {
                    async move {
                        let identifier = <GruntIdentifier as crate::packets::Identifier<T>>::recv(source, self).await?;
                        match identifier {
                            0x00 => {
                                let msg = <LogonChallengeRequest as crate::packets::Payload<T>>::recv(source, self).await?;
                                Self::handle_logon_challenge_request(self, msg, dest).await
                            }
                            0x01 => {
                                let msg = <LogonProofRequest as crate::packets::Payload<T>>::recv(source, self).await?;
                                Self::handle_logon_proof_request(self, msg, dest).await
                            }
                            _ => Err(::anyhow::anyhow!("Unknown identifier")),
                        }
                    }
                }
            }
        });
    }
}